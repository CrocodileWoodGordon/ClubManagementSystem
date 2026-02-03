use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Detailed fee breakdown for a student in a specific class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBreakdown {
    pub student_id: Uuid,
    pub class_id: Uuid,
    pub material_fee: f64,
    pub lesson_fee: f64,
    pub attendance_count: u32,
    pub remarks: Option<String>,
}
