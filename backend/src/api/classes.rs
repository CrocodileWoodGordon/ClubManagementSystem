use std::fmt::Write;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post, put},
};
use chrono::{NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::ApiState,
    domain::ClassStatus,
    error::AppError,
    services::class_assignment_service::{
        ClassAssignmentInput, ClassAssignmentService, ClassLookupFilters, ClassSummary,
        CreateClassInput, UpdateClassInput,
    },
};

#[derive(Debug, Deserialize)]
pub struct ClassLookupQuery {
    pub term_id: Option<Uuid>,
    pub campus_id: Uuid,
    pub club_id: Uuid,
    pub weekday: u8,
}

#[derive(Debug, Deserialize)]
pub struct CreateClassRequest {
    pub term_id: Option<Uuid>,
    pub campus_id: Uuid,
    pub club_id: Uuid,
    pub weekday: u8,
    pub class_code: String,
    pub start_time: String,
    pub end_time: String,
    pub location: Option<String>,
    pub capacity: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignmentRequest {
    pub term_id: Option<Uuid>,
    pub campus_id: Uuid,
    pub club_id: Uuid,
    pub weekday: u8,
    pub class_id: Option<Uuid>,
    pub enrollment_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct ClassListResponse {
    pub data: Vec<ClassDto>,
}

#[derive(Debug, Serialize)]
pub struct ClassDetailResponse {
    pub data: ClassDto,
}

#[derive(Debug, Serialize)]
pub struct AssignmentResponse {
    pub updated: u64,
}

#[derive(Debug, Serialize)]
pub struct ClassDto {
    pub id: Uuid,
    pub term_id: Uuid,
    pub campus_id: Uuid,
    pub club_id: Uuid,
    pub class_code: String,
    pub weekday: u8,
    pub start_time: String,
    pub end_time: String,
    pub location: Option<String>,
    pub capacity: Option<i32>,
    pub status: ClassStatus,
    pub notes: Option<String>,
    pub assigned_count: i64,
}

async fn list_classes(
    State(state): State<ApiState>,
    Query(query): Query<ClassLookupQuery>,
) -> Result<Json<ClassListResponse>, AppError> {
    let service = ClassAssignmentService::new(&state.pool);
    let filters = ClassLookupFilters {
        term_id: query.term_id,
        campus_id: query.campus_id,
        club_id: query.club_id,
        weekday: query.weekday,
    };
    let classes = service.list_classes(&filters).await?;
    Ok(Json(ClassListResponse {
        data: classes.into_iter().map(ClassDto::from).collect(),
    }))
}

async fn create_class(
    State(state): State<ApiState>,
    Json(payload): Json<CreateClassRequest>,
) -> Result<Json<ClassDetailResponse>, AppError> {
    let service = ClassAssignmentService::new(&state.pool);
    let input = CreateClassInput {
        term_id: payload.term_id,
        campus_id: payload.campus_id,
        club_id: payload.club_id,
        weekday: payload.weekday,
        class_code: payload.class_code,
        start_time: parse_time("start_time", &payload.start_time)?,
        end_time: parse_time("end_time", &payload.end_time)?,
        location: payload.location,
        capacity: payload.capacity,
        notes: payload.notes,
    };
    let class = service.create_class(&input).await?;
    Ok(Json(ClassDetailResponse { data: class.into() }))
}

async fn assign_students(
    State(state): State<ApiState>,
    Json(payload): Json<AssignmentRequest>,
) -> Result<Json<AssignmentResponse>, AppError> {
    let service = ClassAssignmentService::new(&state.pool);
    let input = ClassAssignmentInput {
        term_id: payload.term_id,
        campus_id: payload.campus_id,
        club_id: payload.club_id,
        weekday: payload.weekday,
        class_id: payload.class_id,
        enrollment_ids: payload.enrollment_ids,
    };
    let updated = service.assign_enrollments(&input).await?;
    Ok(Json(AssignmentResponse { updated }))
}

async fn update_class(
    State(state): State<ApiState>,
    Path(class_id): Path<Uuid>,
    Json(payload): Json<CreateClassRequest>,
) -> Result<Json<ClassDetailResponse>, AppError> {
    let service = ClassAssignmentService::new(&state.pool);
    let input = UpdateClassInput {
        class_id,
        term_id: payload.term_id,
        campus_id: payload.campus_id,
        club_id: payload.club_id,
        weekday: payload.weekday,
        class_code: payload.class_code,
        start_time: parse_time("start_time", &payload.start_time)?,
        end_time: parse_time("end_time", &payload.end_time)?,
        location: payload.location,
        capacity: payload.capacity,
        notes: payload.notes,
    };
    let class = service.update_class(&input).await?;
    Ok(Json(ClassDetailResponse { data: class.into() }))
}

impl From<ClassSummary> for ClassDto {
    fn from(summary: ClassSummary) -> Self {
        ClassDto {
            id: summary.id,
            term_id: summary.term_id,
            campus_id: summary.campus_id,
            club_id: summary.club_id,
            class_code: summary.class_code,
            weekday: summary.weekday,
            start_time: format_time(summary.start_time),
            end_time: format_time(summary.end_time),
            location: summary.location,
            capacity: summary.capacity,
            status: summary.status,
            notes: summary.notes,
            assigned_count: summary.assigned_count,
        }
    }
}

fn parse_time(field: &str, value: &str) -> Result<NaiveTime, AppError> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .map_err(|_| AppError::Validation(format!("字段 `{}` 需使用 HH:MM 格式", field)))
}

fn format_time(value: NaiveTime) -> String {
    let mut buf = String::with_capacity(5);
    write!(&mut buf, "{:02}:{:02}", value.hour(), value.minute()).unwrap();
    buf
}

/// Manage class shells and student assignments.
pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/assign", post(assign_students))
        .route("/", get(list_classes).post(create_class))
        .route("/:id", put(update_class))
}
