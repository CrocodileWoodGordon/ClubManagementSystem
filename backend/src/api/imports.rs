use axum::{
    extract::{Multipart, State},
    routing::post,
    Json, Router,
};
use chrono::Datelike;
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    api::ApiState,
    db::DbPool,
    domain::EnrollmentImportOutcome,
    error::AppError,
    services::{ExcelImportService, StudentImportSummary},
};

#[derive(Debug, Serialize)]
struct EnrollmentImportResponse {
    outcomes: Vec<EnrollmentImportOutcome>,
}

async fn import_enrollments(
    State(state): State<ApiState>,
    mut multipart: Multipart,
) -> Result<Json<EnrollmentImportResponse>, AppError> {
    let term = find_active_term(&state.pool).await?;
    let mut service = ExcelImportService::new(&state.pool);
    let outcomes = service
        .ingest_enrollments(term.id, "system", &mut multipart)
        .await?;
    Ok(Json(EnrollmentImportResponse { outcomes }))
}

#[derive(Debug, Serialize)]
struct StudentImportResponse {
    summary: StudentImportSummary,
}

async fn import_students(
    State(state): State<ApiState>,
    mut multipart: Multipart,
) -> Result<Json<StudentImportResponse>, AppError> {
    let term = find_active_term(&state.pool).await?;
    let mut service = ExcelImportService::new(&state.pool);
    let summary = service
        .ingest_students(term.id, term.academic_year, "system", &mut multipart)
        .await?;
    Ok(Json(StudentImportResponse { summary }))
}

struct ActiveTerm {
    id: Uuid,
    academic_year: i16,
}

async fn find_active_term(pool: &DbPool) -> Result<ActiveTerm, AppError> {
    let result = sqlx::query(
        r#"
            SELECT id, start_date
            FROM terms
            WHERE is_active = true
            ORDER BY enrollment_start DESC
            LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    if let Some(row) = result {
        let id: Uuid = row
            .try_get("id")
            .map_err(|err| AppError::Database(err.to_string()))?;
        let start_date: chrono::NaiveDate = row
            .try_get("start_date")
            .map_err(|err| AppError::Database(err.to_string()))?;
        let academic_year = start_date.year() as i16;
        Ok(ActiveTerm { id, academic_year })
    } else {
        Err(AppError::Validation(
            "未找到激活学期，无法导入报名数据".into(),
        ))
    }
}

/// Handles Excel uploads from 问卷星 for both students + enrollments.
pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/enrollments", post(import_enrollments))
        .route("/students", post(import_students))
}
