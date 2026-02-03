use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Concrete class that ties a club to a weekday + batch number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInstance {
    pub id: Option<Uuid>,
    pub club_id: Uuid,
    pub day_of_week: u8,
    pub batch_number: String,
    pub time_slot: String,
    pub location: String,
}
