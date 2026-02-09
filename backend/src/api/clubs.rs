use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, put},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::ApiState,
    error::AppError,
    services::{
        AddMembersRequest as ServiceAddMembersRequest, ClubDto, ClubListFilters, ClubMemberDto,
        ClubMemberFilters, ClubService, ClubUpdateChanges, MembershipEntry, NewClubInput,
    },
};

#[derive(Debug, Deserialize, Default)]
pub struct ClubListQuery {
    pub search: Option<String>,
    pub term_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct ClubListResponse {
    pub data: Vec<ClubDto>,
}

#[derive(Debug, Serialize)]
pub struct ClubDetailResponse {
    pub data: ClubDto,
}

#[derive(Debug, Serialize)]
pub struct ClubMemberListResponse {
    pub data: Vec<ClubMemberDto>,
}

#[derive(Debug, Deserialize)]
pub struct CreateClubRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub material_fee: f64,
    #[serde(default)]
    pub price_per_session: f64,
    #[serde(default = "default_grace_sessions")]
    pub grace_sessions: i16,
}

#[derive(Debug, Deserialize)]
pub struct UpdateClubRequest {
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub material_fee: Option<f64>,
    pub price_per_session: Option<f64>,
    pub grace_sessions: Option<i16>,
}

#[derive(Debug, Deserialize)]
pub struct MemberQuery {
    pub term_id: Uuid,
    pub campus_id: Uuid,
    pub weekday: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct AddMembersRequest {
    pub term_id: Uuid,
    pub campus_id: Uuid,
    pub entries: Vec<MemberEntry>,
}

#[derive(Debug, Deserialize)]
pub struct MemberEntry {
    pub student_id: Uuid,
    pub requested_weekday: u8,
}

fn default_grace_sessions() -> i16 {
    3
}

async fn list_clubs(
    State(state): State<ApiState>,
    Query(query): Query<ClubListQuery>,
) -> Result<Json<ClubListResponse>, AppError> {
    let service = ClubService::new(&state.pool);
    let filters = ClubListFilters {
        search: query.search,
        term_id: query.term_id,
    };
    let clubs = service.list(&filters).await?;
    Ok(Json(ClubListResponse { data: clubs }))
}

async fn create_club(
    State(state): State<ApiState>,
    Json(payload): Json<CreateClubRequest>,
) -> Result<Json<ClubDetailResponse>, AppError> {
    let service = ClubService::new(&state.pool);
    let input = NewClubInput {
        code: payload.code,
        name: payload.name,
        description: payload.description,
        material_fee: payload.material_fee,
        price_per_session: payload.price_per_session,
        grace_sessions: payload.grace_sessions,
    };
    let club = service.create(input).await?;
    Ok(Json(ClubDetailResponse { data: club }))
}

async fn update_club(
    State(state): State<ApiState>,
    Path(club_id): Path<Uuid>,
    Json(payload): Json<UpdateClubRequest>,
) -> Result<Json<ClubDetailResponse>, AppError> {
    let service = ClubService::new(&state.pool);
    let changes = ClubUpdateChanges {
        code: payload.code,
        name: payload.name,
        description: payload.description,
        material_fee: payload.material_fee,
        price_per_session: payload.price_per_session,
        grace_sessions: payload.grace_sessions,
    };
    let club = service.update(club_id, changes).await?;
    Ok(Json(ClubDetailResponse { data: club }))
}

async fn delete_club(
    State(state): State<ApiState>,
    Path(club_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let service = ClubService::new(&state.pool);
    service.delete(club_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_members(
    State(state): State<ApiState>,
    Path(club_id): Path<Uuid>,
    Query(query): Query<MemberQuery>,
) -> Result<Json<ClubMemberListResponse>, AppError> {
    if let Some(weekday) = query.weekday {
        if weekday == 0 || weekday > 7 {
            return Err(AppError::Validation(
                "weekday 需在 1-7 之间（1=周一）".into(),
            ));
        }
    }
    let service = ClubService::new(&state.pool);
    let filters = ClubMemberFilters {
        term_id: query.term_id,
        campus_id: query.campus_id,
        weekday: query.weekday,
    };
    let members = service.list_members(club_id, &filters).await?;
    Ok(Json(ClubMemberListResponse { data: members }))
}

async fn add_members(
    State(state): State<ApiState>,
    Path(club_id): Path<Uuid>,
    Json(payload): Json<AddMembersRequest>,
) -> Result<Json<ClubMemberListResponse>, AppError> {
    let service = ClubService::new(&state.pool);
    let entries = payload
        .entries
        .into_iter()
        .map(|entry| MembershipEntry {
            student_id: entry.student_id,
            requested_weekday: entry.requested_weekday,
        })
        .collect();
    let request = ServiceAddMembersRequest {
        term_id: payload.term_id,
        campus_id: payload.campus_id,
        entries,
    };
    let members = service.add_members(club_id, request).await?;
    Ok(Json(ClubMemberListResponse { data: members }))
}

async fn remove_member(
    State(state): State<ApiState>,
    Path((club_id, enrollment_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let service = ClubService::new(&state.pool);
    service.remove_member(club_id, enrollment_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Routes for managing club definitions and membership.
pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/", get(list_clubs).post(create_club))
        .route("/{id}", put(update_club).delete(delete_club))
        .route("/{id}/members", get(list_members).post(add_members))
        .route("/{id}/members/{enrollment_id}", delete(remove_member))
}
