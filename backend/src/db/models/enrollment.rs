use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EnrollmentRow {
    pub id: Uuid,
    pub student_id: Uuid,
    pub class_id: Option<Uuid>,
    pub status: String,
    pub drop_date: Option<NaiveDate>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
