use crate::domain::FeeBreakdown;
use crate::error::AppError;

#[derive(Debug, Default)]
pub struct BillingService;

impl BillingService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn preview_by_class(
        &self,
        _class_id: uuid::Uuid,
    ) -> Result<Vec<FeeBreakdown>, AppError> {
        Ok(Vec::new())
    }

    pub async fn preview_by_student(
        &self,
        _student_id: uuid::Uuid,
    ) -> Result<Vec<FeeBreakdown>, AppError> {
        Ok(Vec::new())
    }
}
