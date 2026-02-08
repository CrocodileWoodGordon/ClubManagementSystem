use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post, put},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{api::ApiState, error::AppError};

#[derive(Debug, Serialize, FromRow)]
pub struct TermDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub enrollment_start: NaiveDate,
    pub enrollment_end: NaiveDate,
    pub is_active: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CampusDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub short_name: Option<String>,
    pub address: Option<String>,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateTermRequest {
    code: String,
    name: String,
    start_date: NaiveDate,
    end_date: NaiveDate,
    enrollment_start: NaiveDate,
    enrollment_end: NaiveDate,
    #[serde(default)]
    is_active: bool,
}

#[derive(Debug, Deserialize)]
struct UpdateTermRequest {
    code: Option<String>,
    name: Option<String>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    enrollment_start: Option<NaiveDate>,
    enrollment_end: Option<NaiveDate>,
    is_active: Option<bool>,
}

impl UpdateTermRequest {
    fn has_changes(&self) -> bool {
        self.code.is_some()
            || self.name.is_some()
            || self.start_date.is_some()
            || self.end_date.is_some()
            || self.enrollment_start.is_some()
            || self.enrollment_end.is_some()
            || self.is_active.is_some()
    }
}

#[derive(Debug, Deserialize)]
struct UpdateCampusRequest {
    name: Option<String>,
    short_name: Option<String>,
    address: Option<String>,
    contact_name: Option<String>,
    contact_phone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateCampusRequest {
    code: String,
    name: String,
    short_name: Option<String>,
    address: Option<String>,
    contact_name: Option<String>,
    contact_phone: Option<String>,
}

impl UpdateCampusRequest {
    fn has_changes(&self) -> bool {
        self.name.is_some()
            || self.short_name.is_some()
            || self.address.is_some()
            || self.contact_name.is_some()
            || self.contact_phone.is_some()
    }
}

async fn list_terms(State(state): State<ApiState>) -> Result<Json<Vec<TermDto>>, AppError> {
    let items = sqlx::query_as::<_, TermDto>(
        r#"
            SELECT id, code, name, start_date, end_date, enrollment_start, enrollment_end, is_active
            FROM terms
            ORDER BY start_date DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    Ok(Json(items))
}

async fn create_term(
    State(state): State<ApiState>,
    Json(payload): Json<CreateTermRequest>,
) -> Result<Json<TermDto>, AppError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    if payload.is_active {
        sqlx::query("UPDATE terms SET is_active = false WHERE is_active = true")
            .execute(tx.as_mut())
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;
    }

    let row = sqlx::query_as::<_, TermDto>(
        r#"
            INSERT INTO terms (code, name, start_date, end_date, enrollment_start, enrollment_end, is_active)
            VALUES ($1,$2,$3,$4,$5,$6,$7)
            RETURNING id, code, name, start_date, end_date, enrollment_start, enrollment_end, is_active
        "#,
    )
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.start_date)
    .bind(payload.end_date)
    .bind(payload.enrollment_start)
    .bind(payload.enrollment_end)
    .bind(payload.is_active)
    .fetch_one(tx.as_mut())
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    tx.commit()
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    Ok(Json(row))
}

async fn update_term(
    Path(term_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(payload): Json<UpdateTermRequest>,
) -> Result<Json<TermDto>, AppError> {
    if !payload.has_changes() {
        return Err(AppError::Validation("请至少提供一个需要更新的字段".into()));
    }
    if let Some(code) = payload.code.as_ref() {
        if code.trim().is_empty() {
            return Err(AppError::Validation("学期编号不能为空".into()));
        }
    }
    if let Some(name) = payload.name.as_ref() {
        if name.trim().is_empty() {
            return Err(AppError::Validation("学期名称不能为空".into()));
        }
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    let row = sqlx::query_as::<_, TermDto>(
        r#"
            UPDATE terms
            SET code = COALESCE($2, code),
                name = COALESCE($3, name),
                start_date = COALESCE($4, start_date),
                end_date = COALESCE($5, end_date),
                enrollment_start = COALESCE($6, enrollment_start),
                enrollment_end = COALESCE($7, enrollment_end),
                is_active = COALESCE($8, is_active)
            WHERE id = $1
            RETURNING id, code, name, start_date, end_date, enrollment_start, enrollment_end, is_active
        "#,
    )
    .bind(term_id)
    .bind(payload.code.as_deref())
    .bind(payload.name.as_deref())
    .bind(payload.start_date)
    .bind(payload.end_date)
    .bind(payload.enrollment_start)
    .bind(payload.enrollment_end)
    .bind(payload.is_active)
    .fetch_optional(tx.as_mut())
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    let Some(row) = row else {
        return Err(AppError::NotFound("未找到对应学期".into()));
    };

    if matches!(payload.is_active, Some(true)) {
        sqlx::query("UPDATE terms SET is_active = false WHERE id <> $1 AND is_active = true")
            .bind(term_id)
            .execute(tx.as_mut())
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    Ok(Json(row))
}

async fn delete_term(
    Path(term_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<StatusCode, AppError> {
    let is_active = sqlx::query_scalar::<_, bool>("SELECT is_active FROM terms WHERE id = $1")
        .bind(term_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    let Some(is_active) = is_active else {
        return Err(AppError::NotFound("未找到对应学期".into()));
    };

    if is_active {
        return Err(AppError::Validation(
            "请先切换当前学期后再删除该学期".into(),
        ));
    }

    sqlx::query("DELETE FROM terms WHERE id = $1")
        .bind(term_id)
        .execute(&state.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn activate_term(
    Path(term_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<Json<TermDto>, AppError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    let row = sqlx::query_as::<_, TermDto>(
        r#"
            UPDATE terms
            SET is_active = true
            WHERE id = $1
            RETURNING id, code, name, start_date, end_date, enrollment_start, enrollment_end, is_active
        "#,
    )
    .bind(term_id)
    .fetch_optional(tx.as_mut())
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    let Some(row) = row else {
        return Err(AppError::NotFound("未找到对应学期".into()));
    };

    sqlx::query("UPDATE terms SET is_active = false WHERE id <> $1 AND is_active = true")
        .bind(term_id)
        .execute(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    tx.commit()
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    Ok(Json(row))
}

async fn list_campuses(State(state): State<ApiState>) -> Result<Json<Vec<CampusDto>>, AppError> {
    let items = sqlx::query_as::<_, CampusDto>(
        r#"
            SELECT id, code, name, short_name, address, contact_name, contact_phone
            FROM campuses
            ORDER BY name ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    Ok(Json(items))
}

async fn create_campus(
    State(state): State<ApiState>,
    Json(payload): Json<CreateCampusRequest>,
) -> Result<Json<CampusDto>, AppError> {
    if payload.code.trim().is_empty() || payload.name.trim().is_empty() {
        return Err(AppError::Validation("请提供校区编号与名称".into()));
    }

    let row = sqlx::query_as::<_, CampusDto>(
        r#"
            INSERT INTO campuses (code, name, short_name, address, contact_name, contact_phone)
            VALUES ($1,$2,$3,$4,$5,$6)
            RETURNING id, code, name, short_name, address, contact_name, contact_phone
        "#,
    )
    .bind(payload.code.trim())
    .bind(payload.name.trim())
    .bind(payload.short_name.as_deref())
    .bind(payload.address.as_deref())
    .bind(payload.contact_name.as_deref())
    .bind(payload.contact_phone.as_deref())
    .fetch_one(&state.pool)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    Ok(Json(row))
}

async fn update_campus(
    Path(campus_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(payload): Json<UpdateCampusRequest>,
) -> Result<Json<CampusDto>, AppError> {
    if !payload.has_changes() {
        return Err(AppError::Validation("请至少提供一个需要更新的字段".into()));
    }

    let row = sqlx::query_as::<_, CampusDto>(
        r#"
            UPDATE campuses
            SET name = COALESCE($2, name),
                short_name = COALESCE($3, short_name),
                address = COALESCE($4, address),
                contact_name = COALESCE($5, contact_name),
                contact_phone = COALESCE($6, contact_phone)
            WHERE id = $1
            RETURNING id, code, name, short_name, address, contact_name, contact_phone
        "#,
    )
    .bind(campus_id)
    .bind(payload.name.as_deref())
    .bind(payload.short_name.as_deref())
    .bind(payload.address.as_deref())
    .bind(payload.contact_name.as_deref())
    .bind(payload.contact_phone.as_deref())
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    let Some(row) = row else {
        return Err(AppError::NotFound("未找到对应校区".into()));
    };

    Ok(Json(row))
}

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/terms", get(list_terms).post(create_term))
        .route("/terms/{id}", put(update_term).delete(delete_term))
        .route("/terms/{id}/activate", post(activate_term))
        .route("/campuses", get(list_campuses).post(create_campus))
        .route("/campuses/{id}", patch(update_campus))
}
