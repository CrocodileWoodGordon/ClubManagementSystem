use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AttendanceRecordRow {
    pub id: Uuid,
    pub student_id: Uuid,
    pub class_id: Uuid,
    pub date: NaiveDate,
    pub status: String,
}
