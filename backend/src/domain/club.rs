use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A high level club definition with pricing meta-data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Club {
    pub id: Uuid,
    pub name: String,
    pub material_fee: f64,
    pub price_per_lesson: f64,
}
