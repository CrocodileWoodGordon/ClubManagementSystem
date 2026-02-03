use axum::extract::Multipart;

use crate::error::AppError;
use crate::utils::excel::ExcelWorkbook;

#[derive(Debug, Default)]
pub struct ExcelImportService;

impl ExcelImportService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn ingest_enrollments(&mut self, _payload: &mut Multipart) -> Result<(), AppError> {
        // Placeholder: parse workbook and convert rows into enrollment drafts.
        Ok(())
    }

    pub async fn ingest_students(&mut self, _workbook: ExcelWorkbook) -> Result<(), AppError> {
        Ok(())
    }
}
