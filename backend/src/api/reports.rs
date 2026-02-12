use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::ApiState, domain::FeeBreakdown, error::AppError,
    services::reporting_service::ReportingService,
};

#[derive(Debug, Deserialize)]
pub struct SettlementQuery {
    pub class_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct BillingQuery {
    pub student_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct SettlementResponse {
    pub data: Vec<FeeBreakdown>,
}

async fn settlement(
    State(state): State<ApiState>,
    Query(query): Query<SettlementQuery>,
) -> Result<Json<SettlementResponse>, AppError> {
    let class_id = query
        .class_id
        .ok_or_else(|| AppError::Validation("请提供 class_id".into()))?;
    let service = ReportingService::new(&state.pool);
    let data = service.preview_settlement(class_id).await?;
    Ok(Json(SettlementResponse { data }))
}

async fn billing(
    State(state): State<ApiState>,
    Query(query): Query<BillingQuery>,
) -> Result<Json<SettlementResponse>, AppError> {
    let student_id = query
        .student_id
        .ok_or_else(|| AppError::Validation("请提供 student_id".into()))?;
    let service = ReportingService::new(&state.pool);
    let data = service.preview_student_bill(student_id).await?;
    Ok(Json(SettlementResponse { data }))
}

/// Financial + roster reports for admins.
pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/settlement", get(settlement))
        .route("/billing", get(billing))
}
