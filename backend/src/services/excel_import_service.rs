use axum::extract::Multipart;
use uuid::Uuid;

use crate::{
    db::DbPool,
    domain::EnrollmentImportOutcome,
    error::AppError,
    services::{
        EnrollmentImportColumns, EnrollmentImportService, ImportPlaceholderService,
        ImportPlaceholderType, StudentImportService, StudentImportSummary,
    },
    utils::excel::ExcelWorkbook,
};

/// 负责从 Multipart 中读取 Excel，并交由 EnrollmentImportService 落库。
#[derive(Debug)]
pub struct ExcelImportService<'a> {
    pool: &'a DbPool,
    enrollment_import: EnrollmentImportService,
    student_import: StudentImportService,
}

impl<'a> ExcelImportService<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self {
            pool,
            enrollment_import: EnrollmentImportService::new(),
            student_import: StudentImportService::new(),
        }
    }

    pub async fn ingest_enrollments(
        &mut self,
        term_id: Uuid,
        created_by: &str,
        payload: &mut Multipart,
    ) -> Result<Vec<EnrollmentImportOutcome>, AppError> {
        let upload = Self::read_enrollment_upload(payload).await?;
        let workbook = ExcelWorkbook::from_bytes(upload.bytes, Some(&upload.filename))?;
        let placeholders = ImportPlaceholderService::new(self.pool)
            .resolved_values(ImportPlaceholderType::Enrollments)
            .await?;
        self.enrollment_import
            .import_workbook(
                self.pool,
                term_id,
                workbook,
                created_by,
                &upload.filename,
                upload.columns,
                placeholders,
            )
            .await
    }

    pub async fn ingest_students(
        &mut self,
        term_id: Uuid,
        academic_year: i16,
        created_by: &str,
        payload: &mut Multipart,
    ) -> Result<StudentImportSummary, AppError> {
        let (bytes, filename) = Self::read_first_file(payload).await?;
        let workbook = ExcelWorkbook::from_bytes(bytes, Some(&filename))?;
        self.student_import
            .import_students(
                self.pool,
                term_id,
                academic_year,
                workbook,
                created_by,
                &filename,
            )
            .await
    }

    async fn read_enrollment_upload(
        payload: &mut Multipart,
    ) -> Result<EnrollmentUploadPayload, AppError> {
        let mut file_bytes: Option<Vec<u8>> = None;
        let mut filename: Option<String> = None;
        let mut columns: Option<EnrollmentImportColumns> = None;

        while let Some(field) = payload
            .next_field()
            .await
            .map_err(|err| AppError::Validation(format!("读取上传字段失败: {}", err)))?
        {
            if field.name() == Some("config") {
                let text = field
                    .text()
                    .await
                    .map_err(|err| AppError::Validation(format!("读取列配置失败: {}", err)))?;
                if text.trim().is_empty() {
                    continue;
                }
                let parsed = EnrollmentImportColumns::from_json(&text)?;
                columns = Some(parsed);
                continue;
            }

            let is_file_field = field.file_name().is_some()
                || field.name().map(|name| name == "file").unwrap_or(false);
            if is_file_field {
                let resolved_name = field
                    .file_name()
                    .map(|name| name.to_string())
                    .unwrap_or_else(|| "enrollments.xlsx".into());
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|err| AppError::Validation(format!("读取 Excel 内容失败: {}", err)))?;
                file_bytes = Some(bytes.to_vec());
                filename = Some(resolved_name);
            }
        }

        let bytes = file_bytes.ok_or_else(|| {
            AppError::Validation("未找到 Excel 文件字段，请确认表单包含 `file`".into())
        })?;

        Ok(EnrollmentUploadPayload {
            bytes,
            filename: filename.unwrap_or_else(|| "enrollments.xlsx".into()),
            columns: columns.unwrap_or_default(),
        })
    }

    async fn read_first_file(payload: &mut Multipart) -> Result<(Vec<u8>, String), AppError> {
        while let Some(field) = payload
            .next_field()
            .await
            .map_err(|err| AppError::Validation(format!("读取上传字段失败: {}", err)))?
        {
            if field.file_name().is_none() && field.name() != Some("file") {
                continue;
            }

            let file_name = field
                .file_name()
                .map(|name| name.to_string())
                .unwrap_or_else(|| "enrollments.xlsx".into());
            let bytes = field
                .bytes()
                .await
                .map_err(|err| AppError::Validation(format!("读取 Excel 内容失败: {}", err)))?;
            return Ok((bytes.to_vec(), file_name));
        }

        Err(AppError::Validation(
            "未找到 Excel 文件字段，请确认表单包含 `file`".into(),
        ))
    }
}

struct EnrollmentUploadPayload {
    bytes: Vec<u8>,
    filename: String,
    columns: EnrollmentImportColumns,
}
