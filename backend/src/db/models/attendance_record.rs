use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AttendanceRecordRow {
    pub id: Uuid,
    pub class_meeting_id: Uuid,
    pub enrollment_id: Uuid,
    pub status: String,
    pub minutes_attended: Option<i32>,
    pub recorded_by: Option<String>,
    pub recorded_at: DateTime<Utc>,
}
