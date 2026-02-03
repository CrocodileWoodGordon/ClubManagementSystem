use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ClubRow {
    pub id: Uuid,
    pub name: String,
    pub material_fee: f64,
    pub price_per_lesson: f64,
}
