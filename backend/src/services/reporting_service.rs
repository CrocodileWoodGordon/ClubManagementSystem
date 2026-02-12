use uuid::Uuid;

use crate::{
    db::DbPool, domain::FeeBreakdown, error::AppError, services::billing_service::BillingService,
};

#[derive(Debug)]
pub struct ReportingService<'a> {
    pool: &'a DbPool,
}

impl<'a> ReportingService<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn preview_settlement(&self, class_id: Uuid) -> Result<Vec<FeeBreakdown>, AppError> {
        BillingService::new(self.pool)
            .preview_by_class(class_id)
            .await
    }

    pub async fn preview_student_bill(
        &self,
        student_id: Uuid,
    ) -> Result<Vec<FeeBreakdown>, AppError> {
        BillingService::new(self.pool)
            .preview_by_student(student_id)
            .await
    }
}
