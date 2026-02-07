use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use serde_json::json;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    db::DbPool,
    domain::{EnrollmentDraft, EnrollmentImportOutcome, EnrollmentImportStatus},
    error::AppError,
    utils::excel::ExcelWorkbook,
};

/// Excel 列配置，支持自定义“学生标识列”与星期列。
#[derive(Debug, Clone)]
pub struct EnrollmentImportColumns {
    pub student_identifier_column: usize,
    pub weekday_columns: Vec<(u8, usize)>,
}

impl Default for EnrollmentImportColumns {
    fn default() -> Self {
        Self {
            student_identifier_column: column_label_to_index("E").unwrap_or(4),
            weekday_columns: vec![
                (1, column_label_to_index("H").unwrap_or(7)),
                (2, column_label_to_index("I").unwrap_or(8)),
                (3, column_label_to_index("J").unwrap_or(9)),
                (4, column_label_to_index("K").unwrap_or(10)),
                (5, column_label_to_index("L").unwrap_or(11)),
            ],
        }
    }
}

impl EnrollmentImportColumns {
    pub fn from_json(value: &str) -> Result<Self, AppError> {
        let raw: RawColumnConfig = serde_json::from_str(value)
            .map_err(|err| AppError::Validation(format!("列配置 JSON 解析失败: {}", err)))?;
        Self::from_raw(raw)
    }

    fn from_raw(raw: RawColumnConfig) -> Result<Self, AppError> {
        let mut result = Self::default();

        if let Some(column_ref) = raw.student_column {
            result.student_identifier_column = column_ref
                .to_index()
                .map_err(|msg| AppError::Validation(msg))?;
        }

        if let Some(map) = raw.weekday_columns {
            let mut resolved = Vec::new();
            for (day_str, column_ref) in map {
                let weekday = day_str.parse::<u8>().map_err(|_| {
                    AppError::Validation(format!("星期键 `{}` 不是有效数字 (1-7)", day_str))
                })?;
                if !(1..=7).contains(&weekday) {
                    return Err(AppError::Validation(format!(
                        "星期键 `{}` 超出 1~7 范围",
                        weekday
                    )));
                }
                let column_index = column_ref
                    .to_index()
                    .map_err(|msg| AppError::Validation(msg))?;
                resolved.push((weekday, column_index));
            }
            resolved.sort_by_key(|(weekday, _)| *weekday);
            result.weekday_columns = resolved;
        }

        Ok(result)
    }
}

#[derive(Debug, Clone)]
struct PlaceholderLookup {
    normalized: HashSet<String>,
}

impl PlaceholderLookup {
    fn new(values: &[String]) -> Self {
        let normalized = values
            .iter()
            .map(|value| normalize_placeholder_key(value))
            .filter(|value| !value.is_empty())
            .collect();
        Self { normalized }
    }

    fn is_placeholder(&self, value: &str) -> bool {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return true;
        }
        let normalized = normalize_placeholder_key(trimmed);
        self.normalized.contains(&normalized)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawColumnConfig {
    #[serde(default)]
    student_column: Option<ColumnRef>,
    #[serde(default)]
    weekday_columns: Option<HashMap<String, ColumnRef>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ColumnRef {
    Letter(String),
    Index(u32),
}

impl ColumnRef {
    fn to_index(&self) -> Result<usize, String> {
        match self {
            ColumnRef::Letter(letter) => column_label_to_index(letter).ok_or_else(|| {
                format!("无法解析列字母 `{}`，仅支持 Excel 列名如 A、B...AA", letter)
            }),
            ColumnRef::Index(index) => {
                if *index == 0 {
                    Err("列索引最小为 1".into())
                } else {
                    Ok((*index as usize) - 1)
                }
            }
        }
    }
}

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
        columns: EnrollmentImportColumns,
        placeholders: Vec<String>,
    ) -> Result<Vec<EnrollmentImportOutcome>, AppError> {
        let placeholder_lookup = PlaceholderLookup::new(&placeholders);
        let (drafts, mut outcomes) =
            parse_workbook(term_id, &workbook, &columns, &placeholder_lookup);
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
        let mut club_index = ClubIndex::load(&mut tx, term_id).await?;
        let mut seen_pairs = HashSet::new();
        let mut success_rows = 0;
        let mut any_failures = !outcomes.is_empty();

        for draft in drafts {
            let outcome = process_single_draft(
                &mut tx,
                job_id,
                draft,
                &student_index,
                &mut club_index,
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
    by_compound: HashMap<String, StudentRecord>,
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
        let mut by_compound = HashMap::new();

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

            let compound_key = normalize_key(&format!("{}{}", homeroom, full_name));
            by_compound.insert(compound_key, record.clone());

            if let Some(code) = student_code {
                by_code.insert(normalize_key(&code), record.clone());
            }
        }

        Ok(Self {
            by_name,
            by_code,
            by_compound,
        })
    }

    fn find(&self, draft: &EnrollmentDraft) -> Option<&StudentRecord> {
        if let Some(code) = &draft.student_code {
            if let Some(record) = self.by_code.get(&normalize_key(code)) {
                return Some(record);
            }
        }

        if !draft.homeroom_display_name.is_empty() && !draft.student_full_name.is_empty() {
            let key = format!(
                "{}::{}",
                normalize_key(&draft.homeroom_display_name),
                normalize_key(&draft.student_full_name)
            );
            if let Some(record) = self.by_name.get(&key) {
                return Some(record);
            }
        }

        if !draft.raw_identifier.is_empty() {
            let compound_key = normalize_key(&draft.raw_identifier);
            if let Some(record) = self.by_compound.get(&compound_key) {
                return Some(record);
            }
        }

        None
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
    term_id: Uuid,
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

        Ok(Self { term_id, by_key })
    }

    async fn resolve(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        campus_id: Uuid,
        raw: &str,
    ) -> Result<ClubRecord, AppError> {
        if let Some(record) = self.find_cached(raw, campus_id) {
            return Ok(record);
        }

        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation(
                "社团列存在空值，无法创建报名记录".into(),
            ));
        }

        let club_row = sqlx::query(
            r#"
                INSERT INTO clubs (code, name)
                VALUES ($1,$2)
                ON CONFLICT (name)
                DO UPDATE SET name = EXCLUDED.name
                RETURNING id, name, code
            "#,
        )
        .bind(generate_club_code(trimmed))
        .bind(trimmed)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let club_id: Uuid = club_row
            .try_get("id")
            .map_err(|err| AppError::Database(err.to_string()))?;
        let club_name: String = club_row
            .try_get("name")
            .map_err(|err| AppError::Database(err.to_string()))?;
        let club_code: String = club_row
            .try_get("code")
            .map_err(|err| AppError::Database(err.to_string()))?;

        sqlx::query(
            r#"
                INSERT INTO club_terms (term_id, campus_id, club_id, material_fee, price_per_session)
                VALUES ($1,$2,$3,0,0)
                ON CONFLICT (term_id, campus_id, club_id)
                DO NOTHING
            "#,
        )
        .bind(self.term_id)
        .bind(campus_id)
        .bind(club_id)
        .execute(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let record = ClubRecord {
            id: club_id,
            name: club_name,
            code: club_code,
            campus_id,
        };

        insert_club_record(&mut self.by_key, &record.code, record.clone());
        insert_club_record(&mut self.by_key, &record.name, record.clone());
        insert_club_record(&mut self.by_key, trimmed, record.clone());

        Ok(record)
    }

    fn find_cached(&self, raw: &str, campus_id: Uuid) -> Option<ClubRecord> {
        self.by_key
            .get(&normalize_key(raw))
            .and_then(|records| records.iter().find(|record| record.campus_id == campus_id))
            .cloned()
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

fn generate_club_code(raw: &str) -> String {
    let ascii_prefix: String = raw
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(6)
        .collect();
    let random = Uuid::new_v4().simple().to_string();
    if ascii_prefix.is_empty() {
        format!("auto_{}", random)
    } else {
        format!("auto_{}_{}", ascii_prefix.to_lowercase(), &random[..8])
    }
}

fn column_label_to_index(label: &str) -> Option<usize> {
    let trimmed = label.trim().to_uppercase();
    if trimmed.is_empty() {
        return None;
    }

    let mut value: usize = 0;
    for ch in trimmed.chars() {
        if !('A'..='Z').contains(&ch) {
            return None;
        }
        let digit = (ch as u8 - b'A' + 1) as usize;
        value = value * 26 + digit;
    }

    value.checked_sub(1)
}

fn parse_workbook(
    term_id: Uuid,
    workbook: &ExcelWorkbook,
    columns: &EnrollmentImportColumns,
    placeholders: &PlaceholderLookup,
) -> (Vec<EnrollmentDraft>, Vec<EnrollmentImportOutcome>) {
    let sheet = workbook.primary_sheet();
    let mut drafts = Vec::new();
    let failures = Vec::new();

    if sheet.rows.len() <= 1 {
        return (drafts, failures);
    }

    for (row_index, row) in sheet.rows.iter().enumerate().skip(1) {
        let row_number = (row_index + 1) as u32;
        let identifier = row
            .get(columns.student_identifier_column)
            .map(|cell| cell.trim())
            .unwrap_or("");
        if identifier.is_empty() {
            continue;
        }

        let (student_code, remainder) = extract_student_code(identifier);
        let (homeroom, student_name) = split_homeroom_and_name(&remainder)
            .unwrap_or_else(|| (String::new(), remainder.trim().to_string()));

        for &(weekday, col_index) in &columns.weekday_columns {
            if col_index >= row.len() {
                continue;
            }
            let value = row
                .get(col_index)
                .map(|cell| cell.trim())
                .unwrap_or_default();
            if placeholders.is_placeholder(value) {
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
                raw_identifier: identifier.to_string(),
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
    club_index: &mut ClubIndex,
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

    let club = match club_index
        .resolve(tx, student.campus_id, &draft.club_lookup_value)
        .await
    {
        Ok(record) => record,
        Err(AppError::Validation(message)) => {
            return Ok(EnrollmentImportOutcome {
                source_row: draft.source_row,
                draft: Some(draft),
                status: EnrollmentImportStatus::Failed,
                enrollment_id: None,
                message: Some(message),
            });
        }
        Err(other) => return Err(other),
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

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut ascii_prefix = false;
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_ascii_alphanumeric() {
            ascii_prefix = true;
            continue;
        }
        if ascii_prefix {
            let (left, right) = trimmed.split_at(idx);
            let homeroom = left.trim();
            let student = right.trim();
            if !homeroom.is_empty() && !student.is_empty() {
                return Some((homeroom.to_string(), student.to_string()));
            }
        }
        break;
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

fn normalize_placeholder_key(value: &str) -> String {
    value.trim().to_lowercase()
}
