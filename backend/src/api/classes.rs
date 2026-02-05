use axum::{
    Json, Router,
    extract::Query,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::ApiState;
use crate::domain::{ClassInstance, Enrollment};
use crate::services::class_assignment_service::ClassAssignmentService;

#[derive(Debug, Deserialize)]
pub struct PendingQuery {
    pub club_id: Option<Uuid>,
    pub day_of_week: Option<u8>,
}

#[derive(Debug, Serialize)]
pub struct PendingResponse {
    pub students: Vec<Enrollment>,
}

#[derive(Debug, Deserialize)]
pub struct AssignmentRequest {
    pub student_ids: Vec<Uuid>,
    pub class: ClassInstance,
}

async fn list_pending_students(Query(_filter): Query<PendingQuery>) -> Json<PendingResponse> {
    Json(PendingResponse {
        students: Vec::new(),
    })
}

async fn assign_students(Json(payload): Json<AssignmentRequest>) -> StatusCode {
    let service = ClassAssignmentService::new();
    let _ = service
        .batch_assign(payload.student_ids, payload.class)
        .await;
    StatusCode::ACCEPTED
}

/// Manage class shells and student assignments.
pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/pending", get(list_pending_students))
        .route("/assign", post(assign_students))
}
