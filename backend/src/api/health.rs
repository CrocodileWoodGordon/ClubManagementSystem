use axum::{Json, Router, routing::get};
use serde::Serialize;

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
pub fn router() -> Router {
    Router::new().route("/health", get(health_check))
}
