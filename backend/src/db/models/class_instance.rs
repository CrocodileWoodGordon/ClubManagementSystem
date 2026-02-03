use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ClassInstanceRow {
    pub id: Uuid,
    pub club_id: Uuid,
    pub day_of_week: i16,
    pub batch_number: String,
    pub time_slot: String,
    pub location: String,
}
