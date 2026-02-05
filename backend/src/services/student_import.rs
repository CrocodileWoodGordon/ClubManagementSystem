use std::collections::HashMap;

use serde::Serialize;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{db::DbPool, error::AppError, utils::excel::ExcelWorkbook};

#[derive(Debug, Serialize)]
pub struct StudentImportSummary {
    pub job_id: Uuid,
    pub total_rows: i32,
    pub success_rows: i32,
    pub skipped_rows: i32,
    pub errors: Vec<StudentImportError>,
}

#[derive(Debug, Serialize)]
pub struct StudentImportError {
    pub row: u32,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct StudentImportService;

impl StudentImportService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn import_students(
        &self,
        pool: &DbPool,
        term_id: Uuid,
        academic_year: i16,
        workbook: ExcelWorkbook,
        created_by: &str,
        source_filename: &str,
    ) -> Result<StudentImportSummary, AppError> {
        let drafts = parse_workbook(&workbook);
        if drafts.is_empty() {
            return Err(AppError::Validation(
                "Excel 文件为空，未发现学生数据".into(),
            ));
        }

        let mut tx = pool
            .begin()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        let job_row = sqlx::query(
            r#"
                INSERT INTO import_jobs (term_id, job_type, source_filename, status, total_rows, success_rows, created_by)
                VALUES ($1,'STUDENTS',$2,'PROCESSING',$3,0,$4)
                RETURNING id
            "#,
        )
        .bind(term_id)
        .bind(source_filename)
        .bind(drafts.len() as i32)
        .bind(created_by)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let job_id: Uuid = job_row
            .try_get("id")
            .map_err(|err| AppError::Database(err.to_string()))?;

        let campus_index = CampusIndex::load(&mut tx).await?;
        let mut homerooms = HomeroomCache::new(academic_year);
        let mut summary = StudentImportSummary {
            job_id,
            total_rows: drafts.len() as i32,
            success_rows: 0,
            skipped_rows: 0,
            errors: Vec::new(),
        };

        for draft in &drafts {
            match process_single_draft(&mut tx, draft, &campus_index, &mut homerooms).await {
                Ok(ProcessOutcome::Inserted) => summary.success_rows += 1,
                Ok(ProcessOutcome::Skipped) => summary.skipped_rows += 1,
                Err(AppError::Validation(message)) => {
                    summary.errors.push(StudentImportError {
                        row: draft.row_number,
                        message: message.clone(),
                    });
                    record_job_error(&mut tx, job_id, &message, draft.row_number).await?;
                }
                Err(other) => return Err(other),
            }
        }

        let status = if summary.errors.is_empty() {
            "COMPLETED"
        } else {
            "FAILED"
        };
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
        .bind(summary.success_rows)
        .bind(status)
        .execute(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        tx.commit()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        Ok(summary)
    }
}

async fn process_single_draft(
    tx: &mut Transaction<'_, Postgres>,
    draft: &StudentDraft,
    campus_index: &CampusIndex,
    homerooms: &mut HomeroomCache,
) -> Result<ProcessOutcome, AppError> {
    if draft.campus_value.is_empty() {
        return Err(AppError::Validation("校区列为空，无法导入".into()));
    }
    if draft.class_label.is_empty() {
        return Err(AppError::Validation("班级列为空，无法导入".into()));
    }
    if draft.student_name.is_empty() {
        return Err(AppError::Validation("姓名列为空，无法导入".into()));
    }

    let campus = campus_index
        .find(&draft.campus_value)
        .ok_or_else(|| AppError::Validation(format!("无法匹配校区 `{}`", draft.campus_value)))?;

    let homeroom_id = homerooms
        .get_or_create(tx, campus.id, &draft.class_label)
        .await?;

    let row = sqlx::query(
        r#"
            INSERT INTO students (full_name, homeroom_id)
            VALUES ($1,$2)
            ON CONFLICT ON CONSTRAINT ux_students_active_name DO NOTHING
            RETURNING id
        "#,
    )
    .bind(&draft.student_name)
    .bind(homeroom_id)
    .fetch_optional(tx.as_mut())
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    if row.is_some() {
        Ok(ProcessOutcome::Inserted)
    } else {
        Ok(ProcessOutcome::Skipped)
    }
}

async fn record_job_error(
    tx: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
    message: &str,
    row_number: u32,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
            INSERT INTO import_job_errors (job_id, row_number, error_message)
            VALUES ($1,$2,$3)
        "#,
    )
    .bind(job_id)
    .bind(row_number as i32)
    .bind(message)
    .execute(tx.as_mut())
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;
    Ok(())
}

#[derive(Clone, Debug)]
struct StudentDraft {
    row_number: u32,
    campus_value: String,
    class_label: String,
    student_name: String,
}

fn parse_workbook(workbook: &ExcelWorkbook) -> Vec<StudentDraft> {
    let sheet = workbook.primary_sheet();
    let mut drafts = Vec::new();
    if sheet.rows.len() <= 1 {
        return drafts;
    }

    for (row_index, row) in sheet.rows.iter().enumerate().skip(1) {
        let campus_value = row.get(0).map(|cell| cell.trim()).unwrap_or("").to_string();
        let class_label = row.get(1).map(|cell| cell.trim()).unwrap_or("").to_string();
        let student_name = row.get(2).map(|cell| cell.trim()).unwrap_or("").to_string();
        if campus_value.is_empty() && class_label.is_empty() && student_name.is_empty() {
            continue;
        }

        drafts.push(StudentDraft {
            row_number: (row_index + 1) as u32,
            campus_value,
            class_label,
            student_name,
        });
    }

    drafts
}

#[derive(Clone, Debug)]
struct CampusRecord {
    id: Uuid,
}

struct CampusIndex {
    by_key: HashMap<String, CampusRecord>,
}

impl CampusIndex {
    async fn load(tx: &mut Transaction<'_, Postgres>) -> Result<Self, AppError> {
        let rows = sqlx::query(
            r#"
                SELECT id, code, name
                FROM campuses
            "#,
        )
        .fetch_all(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let mut by_key = HashMap::new();
        for row in rows {
            let id: Uuid = row
                .try_get("id")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let code: String = row
                .try_get("code")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let name: String = row
                .try_get("name")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let record = CampusRecord { id };
            by_key.insert(normalize_key(&code), record.clone());
            by_key.insert(normalize_key(&name), record.clone());
        }

        if by_key.is_empty() {
            return Err(AppError::Validation(
                "请先在系统中配置校区 (campuses)".into(),
            ));
        }

        Ok(Self { by_key })
    }

    fn find(&self, raw: &str) -> Option<&CampusRecord> {
        self.by_key.get(&normalize_key(raw))
    }
}

struct HomeroomCache {
    academic_year: i16,
    cache: HashMap<String, Uuid>,
}

impl HomeroomCache {
    fn new(academic_year: i16) -> Self {
        Self {
            academic_year,
            cache: HashMap::new(),
        }
    }

    async fn get_or_create(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        campus_id: Uuid,
        raw: &str,
    ) -> Result<Uuid, AppError> {
        let key = format!("{}::{}::{}", campus_id, self.academic_year, raw.trim());
        if let Some(id) = self.cache.get(&key) {
            return Ok(*id);
        }

        let meta = derive_homeroom_meta(raw);
        let row = sqlx::query(
            r#"
                INSERT INTO homerooms (campus_id, academic_year, grade_label, class_label, display_name)
                VALUES ($1,$2,$3,$4,$5)
                ON CONFLICT (campus_id, academic_year, display_name)
                DO UPDATE SET grade_label = EXCLUDED.grade_label,
                              class_label = EXCLUDED.class_label
                RETURNING id
            "#,
        )
        .bind(campus_id)
        .bind(self.academic_year)
        .bind(meta.grade_label)
        .bind(meta.class_label)
        .bind(meta.display_name)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let id: Uuid = row
            .try_get("id")
            .map_err(|err| AppError::Database(err.to_string()))?;
        self.cache.insert(key, id);
        Ok(id)
    }
}

struct HomeroomMeta {
    display_name: String,
    grade_label: String,
    class_label: String,
}

fn derive_homeroom_meta(raw: &str) -> HomeroomMeta {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return HomeroomMeta {
            display_name: String::new(),
            grade_label: String::new(),
            class_label: String::new(),
        };
    }

    let digits: String = trimmed
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if digits.len() >= 1 && digits.len() < trimmed.len() {
        let grade_digit = digits.chars().next().unwrap();
        let class_part = trimmed[digits.len()..].trim_start_matches('0');
        let class_label = if class_part.is_empty() {
            "0班".to_string()
        } else {
            format!("{}班", class_part)
        };
        return HomeroomMeta {
            display_name: trimmed.to_string(),
            grade_label: format!("{}年级", grade_digit),
            class_label,
        };
    }

    if trimmed.len() >= 2 && trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        let grade_digit = trimmed.chars().next().unwrap();
        let class_digits = &trimmed[1..];
        let trimmed_class = class_digits.trim_start_matches('0');
        let class_label = if trimmed_class.is_empty() {
            format!("{}班", class_digits)
        } else {
            format!("{}班", trimmed_class)
        };
        return HomeroomMeta {
            display_name: trimmed.to_string(),
            grade_label: format!("{}年级", grade_digit),
            class_label,
        };
    }

    HomeroomMeta {
        display_name: trimmed.to_string(),
        grade_label: trimmed.to_string(),
        class_label: trimmed.to_string(),
    }
}

fn normalize_key(value: &str) -> String {
    value.trim().to_lowercase()
}

enum ProcessOutcome {
    Inserted,
    Skipped,
}
