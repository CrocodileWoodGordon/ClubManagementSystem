use crate::domain::FeeBreakdown;
use crate::error::AppError;
use crate::services::billing_service::BillingService;

#[derive(Debug, Default)]
pub struct ReportingService;

impl ReportingService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn preview_settlement(&self) -> Result<Vec<FeeBreakdown>, AppError> {
        let billing = BillingService::new();
        // Delegates to billing layer; future versions will join attendance + enrollment info.
        billing.preview_by_class(uuid::Uuid::nil()).await
    }
}
