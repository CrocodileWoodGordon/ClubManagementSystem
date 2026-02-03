use uuid::Uuid;

use crate::domain::{Enrollment, EnrollmentStatus};
use crate::error::AppError;

#[derive(Debug, Default)]
pub struct EnrollmentService;

impl EnrollmentService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn list(&self) -> Result<Vec<Enrollment>, AppError> {
        Ok(Vec::new())
    }

    pub async fn update_status_batch(
        &self,
        _enrollment_ids: Vec<Uuid>,
        _status: EnrollmentStatus,
    ) -> Result<(), AppError> {
        Ok(())
    }
}
