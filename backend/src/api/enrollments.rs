use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{Enrollment, EnrollmentStatus};
use crate::services::enrollment_service::EnrollmentService;

#[derive(Debug, Deserialize)]
pub struct UpdateEnrollmentStatusRequest {
    pub enrollment_ids: Vec<Uuid>,
    pub status: EnrollmentStatus,
}

#[derive(Debug, Serialize)]
pub struct EnrollmentListResponse {
    pub data: Vec<Enrollment>,
}

async fn list_pending_enrollments() -> Json<EnrollmentListResponse> {
    Json(EnrollmentListResponse { data: Vec::new() })
}

async fn bulk_update_status(Json(payload): Json<UpdateEnrollmentStatusRequest>) -> StatusCode {
    let service = EnrollmentService::new();
    let _ = service
        .update_status_batch(payload.enrollment_ids, payload.status)
        .await;
    StatusCode::ACCEPTED
}

/// Routes focused on enrollment lifecycle (PENDING/ACTIVE/DROPPED/TRANSFERRED).
pub fn router() -> Router {
    Router::new()
        .route("/pending", get(list_pending_enrollments))
        .route("/status", post(bulk_update_status))
}
