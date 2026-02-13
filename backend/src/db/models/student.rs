use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StudentRow {
    pub id: Uuid,
    pub original_class: String,
    pub name: String,
    pub is_teacher_child: bool,
}
