use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttendanceStatus {
    Present,
    Absent,
    Excused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceRecord {
    pub id: Uuid,
    pub student_id: Uuid,
    pub class_id: Uuid,
    pub date: NaiveDate,
    pub status: AttendanceStatus,
}
