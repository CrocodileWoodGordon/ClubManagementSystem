use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Canonical student information used across services and reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentProfile {
    pub id: Uuid,
    pub name: String,
    pub original_class: String,
    pub is_teacher_child: bool,
}
