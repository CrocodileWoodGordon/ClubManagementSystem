use axum::{Json, Router, extract::Query, routing::get};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::services::reporting_service::ReportingService;

#[derive(Debug, Deserialize)]
pub struct SettlementQuery {
    pub class_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct SettlementResponse {
    pub rows: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct BillingQuery {
    pub student_id: Option<Uuid>,
}

async fn settlement(Query(_query): Query<SettlementQuery>) -> Json<SettlementResponse> {
    let service = ReportingService::new();
    let _ = service.preview_settlement().await;
    Json(SettlementResponse { rows: Vec::new() })
}

async fn billing(Query(_query): Query<BillingQuery>) -> Json<SettlementResponse> {
    Json(SettlementResponse { rows: Vec::new() })
}

/// Financial + roster reports for admins.
pub fn router() -> Router {
    Router::new()
        .route("/settlement", get(settlement))
        .route("/billing", get(billing))
}
