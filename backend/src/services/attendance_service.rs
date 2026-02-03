use crate::domain::AttendanceRecord;
use crate::error::AppError;

#[derive(Debug, Default)]
pub struct AttendanceService;

impl AttendanceService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn record_bulk(&self, _records: Vec<AttendanceRecord>) -> Result<(), AppError> {
        // TODO: persist via repository + handle deduplication.
        Ok(())
    }

    pub async fn generate_sheet(&self, _class_id: uuid::Uuid) -> Result<Vec<u8>, AppError> {
        // TODO: call tasks::attendance_sheet for actual Excel bytes.
        Ok(Vec::new())
    }

    pub async fn list_records(&self) -> Result<Vec<AttendanceRecord>, AppError> {
        Ok(Vec::new())
    }
}
