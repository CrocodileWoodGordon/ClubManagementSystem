use uuid::Uuid;

use crate::domain::{ClassInstance, Enrollment};
use crate::error::AppError;

#[derive(Debug, Default)]
pub struct ClassAssignmentService;

impl ClassAssignmentService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn list_pending(&self) -> Result<Vec<Enrollment>, AppError> {
        Ok(Vec::new())
    }

    pub async fn batch_assign(
        &self,
        _student_ids: Vec<Uuid>,
        _class: ClassInstance,
    ) -> Result<(), AppError> {
        Ok(())
    }
}
