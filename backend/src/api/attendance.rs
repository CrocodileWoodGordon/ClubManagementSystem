use axum::{Json, Router, extract::Path, http::StatusCode, routing::post};
use chrono::NaiveDate;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::{AttendanceRecord, AttendanceStatus};
use crate::services::attendance_service::AttendanceService;

#[derive(Debug, Deserialize)]
pub struct AttendancePayload {
    pub records: Vec<AttendanceRow>,
}

#[derive(Debug, Deserialize)]
pub struct AttendanceRow {
    pub student_id: Uuid,
    pub class_id: Uuid,
    pub date: NaiveDate,
    pub status: String,
}

async fn bulk_upload(Json(payload): Json<AttendancePayload>) -> StatusCode {
    let service = AttendanceService::new();
    let records: Vec<AttendanceRecord> = payload
        .records
        .into_iter()
        .map(|row| AttendanceRecord {
            id: Uuid::new_v4(),
            student_id: row.student_id,
            class_id: row.class_id,
            date: row.date,
            status: match row.status.to_uppercase().as_str() {
                "ABSENT" => AttendanceStatus::Absent,
                "EXCUSED" => AttendanceStatus::Excused,
                _ => AttendanceStatus::Present,
            },
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
pub fn router() -> Router {
    Router::new()
        .route("/bulk", post(bulk_upload))
        .route("/template/:class_id", post(download_template))
}
