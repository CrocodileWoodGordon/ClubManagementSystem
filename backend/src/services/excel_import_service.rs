use axum::extract::Multipart;
use uuid::Uuid;

use crate::{
    db::DbPool, domain::EnrollmentImportOutcome, error::AppError,
    services::EnrollmentImportService, utils::excel::ExcelWorkbook,
};

/// 负责从 Multipart 中读取 Excel，并交由 EnrollmentImportService 落库。
#[derive(Debug)]
pub struct ExcelImportService<'a> {
    pool: &'a DbPool,
    enrollment_import: EnrollmentImportService,
}

impl<'a> ExcelImportService<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self {
            pool,
            enrollment_import: EnrollmentImportService::new(),
        }
    }

    pub async fn ingest_enrollments(
        &mut self,
        term_id: Uuid,
        created_by: &str,
        payload: &mut Multipart,
    ) -> Result<Vec<EnrollmentImportOutcome>, AppError> {
        let (bytes, filename) = Self::read_first_file(payload).await?;
        let workbook = ExcelWorkbook::from_bytes(bytes)?;
        self.enrollment_import
            .import_workbook(self.pool, term_id, workbook, created_by, &filename)
            .await
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
