use axum::{Json, Router, extract::Path, http::StatusCode, routing::post};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::api::ApiState;
use crate::domain::{AttendanceRecord, AttendanceStatus};
use crate::services::attendance_service::AttendanceService;

#[derive(Debug, Deserialize)]
pub struct AttendancePayload {
    pub records: Vec<AttendanceRow>,
}

#[derive(Debug, Deserialize)]
pub struct AttendanceRow {
    pub class_meeting_id: Uuid,
    pub enrollment_id: Uuid,
    pub status: String,
    pub minutes_attended: Option<i32>,
    pub recorded_by: Option<String>,
}

async fn bulk_upload(Json(payload): Json<AttendancePayload>) -> StatusCode {
    let service = AttendanceService::new();
    let records: Vec<AttendanceRecord> = payload
        .records
        .into_iter()
        .map(|row| AttendanceRecord {
            id: Uuid::new_v4(),
            class_meeting_id: row.class_meeting_id,
            enrollment_id: row.enrollment_id,
            status: AttendanceStatus::try_from(row.status.as_str()).unwrap_or(AttendanceStatus::Present),
            minutes_attended: row.minutes_attended,
            recorded_by: row.recorded_by,
            recorded_at: Utc::now(),
        })
        .collect();
    let _ = service.record_bulk(records).await;
    StatusCode::ACCEPTED
}

async fn download_template(Path(_class_id): Path<Uuid>) -> StatusCode {
    // Placeholder: final implementation streams the generated Excel.
    StatusCode::NO_CONTENT
}

/// Attendance upload + template generation endpoints.
pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/bulk", post(bulk_upload))
        .route("/template/{class_id}", post(download_template))
}
