use std::collections::{HashMap, HashSet};

use serde_json::json;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    db::DbPool,
    domain::{EnrollmentDraft, EnrollmentImportOutcome, EnrollmentImportStatus},
    error::AppError,
    utils::excel::ExcelWorkbook,
};

const WEEKDAY_COLUMNS: [(usize, u8); 5] = [
    (1, 1), // Monday
    (2, 2),
    (3, 3),
    (4, 4),
    (5, 5), // Friday
];

/// 核心报名导入服务：负责将 Excel 解析后的报名信息写入数据库。
#[derive(Debug, Default)]
pub struct EnrollmentImportService;

impl EnrollmentImportService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn import_workbook(
        &self,
        pool: &DbPool,
        term_id: Uuid,
        workbook: ExcelWorkbook,
        created_by: &str,
        source_filename: &str,
    ) -> Result<Vec<EnrollmentImportOutcome>, AppError> {
        let (drafts, mut outcomes) = parse_workbook(term_id, &workbook);
        let total_rows = (drafts.len() + outcomes.len()) as i32;

        if total_rows == 0 {
            return Err(AppError::Validation(
                "Excel 文件为空，未发现报名数据".into(),
            ));
        }

        let mut tx = pool
            .begin()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        let job_row = sqlx::query(
            r#"
                INSERT INTO import_jobs (term_id, job_type, source_filename, status, total_rows, success_rows, created_by)
                VALUES ($1,'ENROLLMENTS',$2,'PROCESSING',$3,0,$4)
                RETURNING id
            "#
        )
        .bind(term_id)
        .bind(source_filename)
        .bind(total_rows)
        .bind(created_by)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let job_id: Uuid = job_row
            .try_get("id")
            .map_err(|err| AppError::Database(err.to_string()))?;
        for outcome in outcomes
            .iter()
            .filter(|o| matches!(o.status, EnrollmentImportStatus::Failed))
        {
            record_job_error(&mut tx, job_id, outcome).await?;
        }

        let student_index = StudentIndex::load(&mut tx).await?;
        let club_index = ClubIndex::load(&mut tx, term_id).await?;
        let mut seen_pairs = HashSet::new();
        let mut success_rows = 0;
        let mut any_failures = !outcomes.is_empty();

        for draft in drafts {
            let outcome = process_single_draft(
                &mut tx,
                job_id,
                draft,
                &student_index,
                &club_index,
                &mut seen_pairs,
            )
            .await?;

            if matches!(outcome.status, EnrollmentImportStatus::Created) {
                success_rows += 1;
            } else if matches!(outcome.status, EnrollmentImportStatus::Failed) {
                any_failures = true;
                record_job_error(&mut tx, job_id, &outcome).await?;
            }

            outcomes.push(outcome);
        }

        let status = if any_failures { "FAILED" } else { "COMPLETED" };
        sqlx::query(
            r#"
                UPDATE import_jobs
                SET success_rows = $2,
                    status = $3,
                    finished_at = now()
                WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(success_rows)
        .bind(status)
        .execute(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        tx.commit()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        Ok(outcomes)
    }
}

#[derive(Clone, Debug)]
struct StudentRecord {
    id: Uuid,
    homeroom: String,
    full_name: String,
    student_code: Option<String>,
    campus_id: Uuid,
}

struct StudentIndex {
    by_name: HashMap<String, StudentRecord>,
    by_code: HashMap<String, StudentRecord>,
}

impl StudentIndex {
    async fn load(tx: &mut Transaction<'_, Postgres>) -> Result<Self, AppError> {
        let rows = sqlx::query(
            r#"
                SELECT s.id,
                       s.full_name,
                       s.student_code,
                       h.display_name AS homeroom,
                       h.campus_id
                FROM students s
                INNER JOIN homerooms h ON h.id = s.homeroom_id
                WHERE s.status = 'ACTIVE'
            "#,
        )
        .fetch_all(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let mut by_name = HashMap::new();
        let mut by_code = HashMap::new();

        for row in rows {
            let id: Uuid = row
                .try_get("id")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let full_name: String = row
                .try_get("full_name")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let homeroom: String = row
                .try_get("homeroom")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let campus_id: Uuid = row
                .try_get("campus_id")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let student_code: Option<String> = row
                .try_get("student_code")
                .map_err(|err| AppError::Database(err.to_string()))?;

            let record = StudentRecord {
                id,
                homeroom: homeroom.clone(),
                full_name: full_name.clone(),
                student_code: student_code.clone(),
                campus_id,
            };
            let key = format!(
                "{}::{}",
                normalize_key(&homeroom),
                normalize_key(&full_name)
            );
            by_name.insert(key, record.clone());

            if let Some(code) = student_code {
                by_code.insert(normalize_key(&code), record.clone());
            }
        }

        Ok(Self { by_name, by_code })
    }

    fn find(&self, draft: &EnrollmentDraft) -> Option<&StudentRecord> {
        if let Some(code) = &draft.student_code {
            if let Some(record) = self.by_code.get(&normalize_key(code)) {
                return Some(record);
            }
        }

        let key = format!(
            "{}::{}",
            normalize_key(&draft.homeroom_display_name),
            normalize_key(&draft.student_full_name)
        );
        self.by_name.get(&key)
    }
}

#[derive(Clone, Debug)]
struct ClubRecord {
    id: Uuid,
    name: String,
    code: String,
    campus_id: Uuid,
}

struct ClubIndex {
    by_key: HashMap<String, Vec<ClubRecord>>,
}

impl ClubIndex {
    async fn load(tx: &mut Transaction<'_, Postgres>, term_id: Uuid) -> Result<Self, AppError> {
        let rows = sqlx::query(
            r#"
                SELECT c.id, c.name, c.code, ct.campus_id
                FROM clubs c
                INNER JOIN club_terms ct ON ct.club_id = c.id
                WHERE ct.term_id = $1
            "#,
        )
        .bind(term_id)
        .fetch_all(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let mut by_key: HashMap<String, Vec<ClubRecord>> = HashMap::new();

        for row in rows {
            let id: Uuid = row
                .try_get("id")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let name: String = row
                .try_get("name")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let code: String = row
                .try_get("code")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let campus_id: Uuid = row
                .try_get("campus_id")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let record = ClubRecord {
                id,
                name: name.clone(),
                code: code.clone(),
                campus_id,
            };
            insert_club_record(&mut by_key, &code, record.clone());
            insert_club_record(&mut by_key, &name, record);
        }

        Ok(Self { by_key })
    }

    fn find(&self, raw: &str, campus_id: Uuid) -> Option<&ClubRecord> {
        self.by_key
            .get(&normalize_key(raw))
            .and_then(|records| records.iter().find(|record| record.campus_id == campus_id))
    }
}

fn insert_club_record(
    map: &mut HashMap<String, Vec<ClubRecord>>,
    raw_key: &str,
    record: ClubRecord,
) {
    let key = normalize_key(raw_key);
    let entry = map.entry(key).or_default();
    let exists = entry
        .iter()
        .any(|existing| existing.campus_id == record.campus_id && existing.id == record.id);
    if !exists {
        entry.push(record);
    }
}

fn parse_workbook(
    term_id: Uuid,
    workbook: &ExcelWorkbook,
) -> (Vec<EnrollmentDraft>, Vec<EnrollmentImportOutcome>) {
    let sheet = workbook.primary_sheet();
    let mut drafts = Vec::new();
    let mut failures = Vec::new();

    if sheet.rows.len() <= 1 {
        return (drafts, failures);
    }

    for (row_index, row) in sheet.rows.iter().enumerate().skip(1) {
        let row_number = (row_index + 1) as u32;
        let identifier = row.get(0).map(|cell| cell.trim()).unwrap_or("");
        if identifier.is_empty() {
            continue;
        }

        let (student_code, remainder) = extract_student_code(identifier);
        let Some((homeroom, student_name)) = split_homeroom_and_name(&remainder) else {
            failures.push(EnrollmentImportOutcome {
                source_row: row_number,
                draft: None,
                status: EnrollmentImportStatus::Failed,
                enrollment_id: None,
                message: Some(format!(
                    "无法解析“年级班级姓名”列：`{}`。请使用“班级 姓名”格式（空格或 - 分隔）。",
                    identifier
                )),
            });
            continue;
        };

        for &(col_index, weekday) in &WEEKDAY_COLUMNS {
            let value = row
                .get(col_index)
                .map(|cell| cell.trim())
                .unwrap_or_default();
            if choice_is_empty(value) {
                continue;
            }

            drafts.push(EnrollmentDraft {
                term_id,
                homeroom_display_name: homeroom.clone(),
                student_full_name: student_name.clone(),
                student_code: student_code.clone(),
                requested_weekday: weekday,
                club_lookup_value: value.to_string(),
                source_row: row_number,
            });
        }
    }

    (drafts, failures)
}

async fn process_single_draft(
    tx: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
    draft: EnrollmentDraft,
    student_index: &StudentIndex,
    club_index: &ClubIndex,
    seen_pairs: &mut HashSet<String>,
) -> Result<EnrollmentImportOutcome, AppError> {
    let student = match student_index.find(&draft) {
        Some(record) => record,
        None => {
            return Ok(EnrollmentImportOutcome {
                source_row: draft.source_row,
                draft: Some(draft),
                status: EnrollmentImportStatus::Failed,
                enrollment_id: None,
                message: Some("未在系统中匹配到对应学生（请检查班级/姓名是否与学生库一致）".into()),
            });
        }
    };

    let club = match club_index.find(&draft.club_lookup_value, student.campus_id) {
        Some(record) => record,
        None => {
            let message = format!(
                "无法匹配社团 `{}`，请确认名称或编码是否存在于当前学期",
                draft.club_lookup_value
            );
            return Ok(EnrollmentImportOutcome {
                source_row: draft.source_row,
                draft: Some(draft),
                status: EnrollmentImportStatus::Failed,
                enrollment_id: None,
                message: Some(message),
            });
        }
    };

    let dedup_key = format!(
        "{}::{}::{}::{}",
        student.id, club.id, draft.requested_weekday, student.campus_id
    );
    if !seen_pairs.insert(dedup_key.clone()) {
        return Ok(EnrollmentImportOutcome {
            source_row: draft.source_row,
            draft: Some(draft),
            status: EnrollmentImportStatus::Skipped,
            enrollment_id: None,
            message: Some("同一学生+社团+星期在 Excel 中出现多次，自动跳过重复行".into()),
        });
    }

    let existing = sqlx::query(
        r#"
            SELECT id
            FROM enrollments
            WHERE term_id = $1
              AND campus_id = $2
              AND student_id = $3
              AND club_id = $4
              AND requested_weekday = $5
              AND status IN ('PENDING','ACTIVE')
        "#,
    )
    .bind(draft.term_id)
    .bind(student.campus_id)
    .bind(student.id)
    .bind(club.id)
    .bind(draft.requested_weekday as i16)
    .fetch_optional(tx.as_mut())
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    if existing.is_some() {
        return Ok(EnrollmentImportOutcome {
            source_row: draft.source_row,
            draft: Some(draft),
            status: EnrollmentImportStatus::Skipped,
            enrollment_id: None,
            message: Some("学生已存在相同社团/星期的有效报名记录，保持原数据不变".into()),
        });
    }

    let inserted_row = sqlx::query(
        r#"
            INSERT INTO enrollments (term_id, campus_id, student_id, club_id, requested_weekday, import_job_id)
            VALUES ($1,$2,$3,$4,$5,$6)
            RETURNING id
        "#,
    )
    .bind(draft.term_id)
    .bind(student.campus_id)
    .bind(student.id)
    .bind(club.id)
    .bind(draft.requested_weekday as i16)
    .bind(job_id)
    .fetch_one(tx.as_mut())
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    let enrollment_id: Uuid = inserted_row
        .try_get("id")
        .map_err(|err| AppError::Database(err.to_string()))?;

    Ok(EnrollmentImportOutcome {
        source_row: draft.source_row,
        draft: Some(draft),
        status: EnrollmentImportStatus::Created,
        enrollment_id: Some(enrollment_id),
        message: None,
    })
}

async fn record_job_error(
    tx: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
    outcome: &EnrollmentImportOutcome,
) -> Result<(), AppError> {
    let raw_payload = outcome
        .draft
        .as_ref()
        .map(|draft| serde_json::to_value(draft).unwrap_or_else(|_| json!({})))
        .unwrap_or_else(|| json!({}));

    let message = outcome
        .message
        .as_deref()
        .unwrap_or("未知错误，请检查 Excel 数据");

    sqlx::query(
        r#"
            INSERT INTO import_job_errors (job_id, row_number, error_message, raw_payload)
            VALUES ($1,$2,$3,$4)
        "#,
    )
    .bind(job_id)
    .bind(outcome.source_row as i32)
    .bind(message)
    .bind(raw_payload)
    .execute(tx.as_mut())
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    Ok(())
}

fn extract_student_code(value: &str) -> (Option<String>, String) {
    let trimmed = value.trim();
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let code = rest[..end].trim();
            let remainder = rest[end + 1..].trim().to_string();
            if !code.is_empty() {
                return (Some(code.to_string()), remainder);
            }
            return (None, remainder);
        }
    }
    (None, trimmed.to_string())
}

fn split_homeroom_and_name(value: &str) -> Option<(String, String)> {
    let separators = [' ', '　', '-', '－', '_', ':', '：', '/', '|'];
    for sep in separators {
        if let Some(idx) = value.rfind(sep) {
            let (left, right) = value.split_at(idx);
            let homeroom = left.trim();
            let mut student = right.trim_start_matches(sep).trim();
            if student.is_empty() {
                student = right.trim();
            }
            if !homeroom.is_empty() && !student.is_empty() {
                return Some((homeroom.to_string(), student.to_string()));
            }
        }
    }
    None
}

fn normalize_key(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

fn choice_is_empty(value: &str) -> bool {
    let normalized = value.trim();
    normalized.is_empty()
        || matches!(
            normalized,
            "-" | "—" | "——" | "无" | "N/A" | "n/a" | "NA" | "na"
        )
}
