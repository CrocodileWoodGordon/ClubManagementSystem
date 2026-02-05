use axum::{
    Router,
    extract::{Multipart, State},
    http::StatusCode,
    routing::post,
};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    api::ApiState, db::DbPool, error::AppError, services::excel_import_service::ExcelImportService,
};

async fn import_enrollments(
    State(state): State<ApiState>,
    mut multipart: Multipart,
) -> Result<StatusCode, AppError> {
    let term_id = find_active_term(&state.pool).await?;
    let mut service = ExcelImportService::new(&state.pool);
    service
        .ingest_enrollments(term_id, "system", &mut multipart)
        .await?;
    Ok(StatusCode::ACCEPTED)
}

async fn find_active_term(pool: &DbPool) -> Result<Uuid, AppError> {
    let result = sqlx::query(
        r#"
            SELECT id
            FROM terms
            WHERE is_active = true
            ORDER BY enrollment_start DESC
            LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    result
        .and_then(|row| row.try_get::<Uuid, _>("id").ok())
        .ok_or_else(|| AppError::Validation("未找到激活学期，无法导入报名数据".into()))
}

/// Handles Excel uploads from 问卷星 for both students + enrollments.
pub fn router() -> Router<ApiState> {
    Router::new().route("/enrollments", post(import_enrollments))
}
