use axum::{Router, extract::Multipart, http::StatusCode, routing::post};

use crate::services::excel_import_service::ExcelImportService;

async fn import_enrollments(mut multipart: Multipart) -> StatusCode {
    let mut service = ExcelImportService::new();
    let _ = service.ingest_enrollments(&mut multipart).await;
    StatusCode::ACCEPTED
}

/// Handles Excel uploads from 问卷星 for both students + enrollments.
pub fn router() -> Router {
    Router::new().route("/enrollments", post(import_enrollments))
}
