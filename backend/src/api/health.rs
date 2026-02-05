use axum::{Json, Router, routing::get};
use serde::Serialize;

use crate::api::ApiState;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    service: String,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        service: "club-management-backend".into(),
    })
}

/// Basic `/health` probe consumed by Docker or monitoring scripts.
pub fn router() -> Router<ApiState> {
    Router::new().route("/health", get(health_check))
}
