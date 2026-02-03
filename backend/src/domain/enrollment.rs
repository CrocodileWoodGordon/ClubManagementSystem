use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Enrollment states drive billing + attendance eligibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EnrollmentStatus {
    Pending,
    Active,
    Dropped,
    Transferred,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrollment {
    pub id: Uuid,
    pub student_id: Uuid,
    pub class_id: Option<Uuid>,
    pub status: EnrollmentStatus,
    pub drop_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}
