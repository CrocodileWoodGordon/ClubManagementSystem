use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::ApiState,
    domain::EnrollmentStatus,
    error::AppError,
    services::{
        EnrollmentFilters,
        EnrollmentService,
        EnrollmentSummaryFilters,
        EnrollmentSummaryRow,
        PendingEnrollmentDto,
    },
};

#[derive(Debug, Deserialize)]
pub struct UpdateEnrollmentStatusRequest {
    pub enrollment_ids: Vec<Uuid>,
    pub status: EnrollmentStatus,
}

#[derive(Debug, Serialize)]
pub struct EnrollmentListResponse {
    pub data: Vec<PendingEnrollmentDto>,
}

#[derive(Debug, Deserialize)]
pub struct EnrollmentListQuery {
    pub term_id: Option<Uuid>,
    pub campus_id: Option<Uuid>,
    pub homeroom: Option<String>,
    #[serde(rename = "club")]
    pub club_name: Option<String>,
    pub weekday: Option<u8>,
    pub student_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EnrollmentSummaryQuery {
    pub term_id: Option<Uuid>,
    pub campus_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct EnrollmentSummaryResponse {
    pub data: Vec<EnrollmentSummaryRow>,
}

async fn list_pending_enrollments(
    State(state): State<ApiState>,
    Query(query): Query<EnrollmentListQuery>,
) -> Result<Json<EnrollmentListResponse>, AppError> {
    let service = EnrollmentService::new(&state.pool);
    let filters = EnrollmentFilters {
        term_id: query.term_id,
        campus_id: query.campus_id,
        homeroom: query.homeroom,
        club_name: query.club_name,
        weekday: query.weekday,
        student_name: query.student_name,
    };
    let data = service.list_pending(&filters).await?;
    Ok(Json(EnrollmentListResponse { data }))
}

async fn summarize_enrollments(
    State(state): State<ApiState>,
    Query(query): Query<EnrollmentSummaryQuery>,
) -> Result<Json<EnrollmentSummaryResponse>, AppError> {
    let service = EnrollmentService::new(&state.pool);
    let filters = EnrollmentSummaryFilters {
        term_id: query.term_id,
        campus_id: query.campus_id,
    };
    let data = service.pending_summary(&filters).await?;
    Ok(Json(EnrollmentSummaryResponse { data }))
}

async fn bulk_update_status(
    State(state): State<ApiState>,
    Json(payload): Json<UpdateEnrollmentStatusRequest>,
) -> Result<StatusCode, AppError> {
    let service = EnrollmentService::new(&state.pool);
    service
        .update_status_batch(&payload.enrollment_ids, payload.status)
        .await?;
    Ok(StatusCode::ACCEPTED)
}

/// Routes focused on enrollment lifecycle (PENDING/ACTIVE/DROPPED/TRANSFERRED).
pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/pending", get(list_pending_enrollments))
        .route("/summary", get(summarize_enrollments))
        .route("/status", post(bulk_update_status))
}
