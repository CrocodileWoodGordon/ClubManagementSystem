use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::ApiState,
    error::AppError,
    services::{
        CloneRosterRequest, CloneRosterResult, HomeroomListFilters, HomeroomRosterDto,
        HomeroomUpdateChanges, NewStudentInput, StudentRecordDto, StudentRosterService,
        UpdateStudentChanges,
    },
};

#[derive(Debug, Deserialize)]
struct HomeroomListQuery {
    term_id: Option<Uuid>,
    campus_id: Option<Uuid>,
    search: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TermQuery {
    term_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct UpdateHomeroomPayload {
    display_name: Option<String>,
    grade_label: Option<String>,
    class_label: Option<String>,
    head_teacher_name: Option<String>,
    head_teacher_phone: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateStudentPayload {
    full_name: String,
    student_code: Option<String>,
    #[serde(default)]
    is_teacher_child: bool,
    primary_guardian_name: Option<String>,
    primary_guardian_phone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateStudentPayload {
    homeroom_id: Option<Uuid>,
    full_name: Option<String>,
    student_code: Option<String>,
    is_teacher_child: Option<bool>,
    primary_guardian_name: Option<String>,
    primary_guardian_phone: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CloneRosterPayload {
    source_term_id: Uuid,
    target_term_id: Uuid,
    campus_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct HomeroomListResponse {
    data: Vec<HomeroomRosterDto>,
}

#[derive(Debug, Serialize)]
struct HomeroomDetailResponse {
    data: HomeroomRosterDto,
}

#[derive(Debug, Serialize)]
struct StudentListResponse {
    data: Vec<StudentRecordDto>,
}

#[derive(Debug, Serialize)]
struct StudentDetailResponse {
    data: StudentRecordDto,
}

#[derive(Debug, Serialize)]
struct CloneRosterResponse {
    data: CloneRosterResult,
}

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/homerooms", get(list_homerooms))
        .route("/homerooms/clone", post(clone_roster))
        .route("/homerooms/{id}", get(get_homeroom).put(update_homeroom))
        .route(
            "/homerooms/{id}/students",
            get(list_students).post(create_student),
        )
        .route("/{id}", put(update_student).delete(delete_student))
}

async fn list_homerooms(
    State(state): State<ApiState>,
    Query(query): Query<HomeroomListQuery>,
) -> Result<Json<HomeroomListResponse>, AppError> {
    let service = StudentRosterService::new(&state.pool);
    let filters = HomeroomListFilters {
        term_id: query.term_id,
        campus_id: query.campus_id,
        search: query.search,
    };
    let data = service.list_homerooms(&filters).await?;
    Ok(Json(HomeroomListResponse { data }))
}

async fn get_homeroom(
    Path(homeroom_id): Path<Uuid>,
    State(state): State<ApiState>,
    Query(query): Query<TermQuery>,
) -> Result<Json<HomeroomDetailResponse>, AppError> {
    let service = StudentRosterService::new(&state.pool);
    let data = service.get_homeroom(homeroom_id, query.term_id).await?;
    Ok(Json(HomeroomDetailResponse { data }))
}

async fn update_homeroom(
    Path(homeroom_id): Path<Uuid>,
    State(state): State<ApiState>,
    Query(query): Query<TermQuery>,
    Json(payload): Json<UpdateHomeroomPayload>,
) -> Result<Json<HomeroomDetailResponse>, AppError> {
    let service = StudentRosterService::new(&state.pool);
    let changes = HomeroomUpdateChanges {
        display_name: sanitize_text(payload.display_name),
        grade_label: sanitize_text(payload.grade_label),
        class_label: sanitize_text(payload.class_label),
        head_teacher_name: sanitize_text(payload.head_teacher_name),
        head_teacher_phone: sanitize_text(payload.head_teacher_phone),
        notes: sanitize_text(payload.notes),
    };
    let data = service
        .update_homeroom(homeroom_id, query.term_id, changes)
        .await?;
    Ok(Json(HomeroomDetailResponse { data }))
}

async fn list_students(
    Path(homeroom_id): Path<Uuid>,
    State(state): State<ApiState>,
    Query(query): Query<TermQuery>,
) -> Result<Json<StudentListResponse>, AppError> {
    let service = StudentRosterService::new(&state.pool);
    let data = service.list_students(homeroom_id, query.term_id).await?;
    Ok(Json(StudentListResponse { data }))
}

async fn create_student(
    Path(homeroom_id): Path<Uuid>,
    State(state): State<ApiState>,
    Query(query): Query<TermQuery>,
    Json(payload): Json<CreateStudentPayload>,
) -> Result<Json<StudentDetailResponse>, AppError> {
    let service = StudentRosterService::new(&state.pool);
    let input = NewStudentInput {
        full_name: payload.full_name,
        student_code: sanitize_text(payload.student_code),
        is_teacher_child: payload.is_teacher_child,
        primary_guardian_name: sanitize_text(payload.primary_guardian_name),
        primary_guardian_phone: sanitize_text(payload.primary_guardian_phone),
    };
    let data = service
        .create_student(homeroom_id, query.term_id, input)
        .await?;
    Ok(Json(StudentDetailResponse { data }))
}

async fn update_student(
    Path(student_id): Path<Uuid>,
    State(state): State<ApiState>,
    Query(query): Query<TermQuery>,
    Json(payload): Json<UpdateStudentPayload>,
) -> Result<Json<StudentDetailResponse>, AppError> {
    let service = StudentRosterService::new(&state.pool);
    let changes = UpdateStudentChanges {
        homeroom_id: payload.homeroom_id,
        full_name: sanitize_text(payload.full_name),
        student_code: sanitize_text(payload.student_code),
        is_teacher_child: payload.is_teacher_child,
        primary_guardian_name: sanitize_text(payload.primary_guardian_name),
        primary_guardian_phone: sanitize_text(payload.primary_guardian_phone),
        status: sanitize_text(payload.status),
    };
    let data = service
        .update_student(student_id, query.term_id, changes)
        .await?;
    Ok(Json(StudentDetailResponse { data }))
}

async fn delete_student(
    Path(student_id): Path<Uuid>,
    State(state): State<ApiState>,
    Query(query): Query<TermQuery>,
) -> Result<(), AppError> {
    let service = StudentRosterService::new(&state.pool);
    service.delete_student(student_id, query.term_id).await
}

async fn clone_roster(
    State(state): State<ApiState>,
    Json(payload): Json<CloneRosterPayload>,
) -> Result<Json<CloneRosterResponse>, AppError> {
    let service = StudentRosterService::new(&state.pool);
    let request = CloneRosterRequest {
        source_term_id: payload.source_term_id,
        target_term_id: payload.target_term_id,
        campus_id: payload.campus_id,
    };
    let data = service.clone_roster(request).await?;
    Ok(Json(CloneRosterResponse { data }))
}

fn sanitize_text(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}
