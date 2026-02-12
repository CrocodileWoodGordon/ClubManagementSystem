use axum::{Json, Router, extract::State, routing::post};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::ApiState,
    domain::{EnrollmentStatus, MaterialFeeState},
    error::AppError,
    services::{
        ClubTransferInput, ClubTransferResult, DropEnrollmentInput, DropEnrollmentResult,
        EnrollmentStatusService, MoveWithinClubInput, MoveWithinClubResult,
    },
};

#[derive(Debug, Deserialize)]
pub struct DropEnrollmentRequest {
    pub enrollment_id: Uuid,
    pub changed_by: String,
    pub reason: Option<String>,
    pub drop_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MoveEnrollmentRequest {
    pub enrollment_id: Uuid,
    pub target_class_id: Option<Uuid>,
    pub changed_by: String,
}

#[derive(Debug, Deserialize)]
pub struct TransferEnrollmentRequest {
    pub source_enrollment_id: Uuid,
    pub target_club_id: Uuid,
    pub target_weekday: u8,
    pub target_class_id: Option<Uuid>,
    pub changed_by: String,
    pub reason: Option<String>,
    pub drop_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DropEnrollmentResponse {
    pub data: DropEnrollmentDto,
}

#[derive(Debug, Serialize)]
pub struct DropEnrollmentDto {
    pub enrollment_id: Uuid,
    pub from_status: EnrollmentStatus,
    pub to_status: EnrollmentStatus,
    pub drop_date: NaiveDate,
    pub waive_tuition_fee: bool,
    pub tuition_grace_applied: bool,
}

#[derive(Debug, Serialize)]
pub struct MoveEnrollmentResponse {
    pub data: MoveEnrollmentDto,
}

#[derive(Debug, Serialize)]
pub struct MoveEnrollmentDto {
    pub enrollment_id: Uuid,
    pub previous_class_id: Option<Uuid>,
    pub new_class_id: Option<Uuid>,
    pub status: EnrollmentStatus,
}

#[derive(Debug, Serialize)]
pub struct TransferEnrollmentResponse {
    pub data: TransferEnrollmentDto,
}

#[derive(Debug, Serialize)]
pub struct TransferEnrollmentDto {
    pub from_enrollment_id: Uuid,
    pub to_enrollment_id: Uuid,
    pub drop_date: NaiveDate,
    pub waived_tuition_fee: bool,
    pub tuition_grace_applied: bool,
    pub carry_over_material_fee: bool,
    pub new_material_fee_state: MaterialFeeState,
}

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/drop", post(drop_enrollment))
        .route("/move", post(move_enrollment))
        .route("/transfer", post(transfer_enrollment))
}

async fn drop_enrollment(
    State(state): State<ApiState>,
    Json(payload): Json<DropEnrollmentRequest>,
) -> Result<Json<DropEnrollmentResponse>, AppError> {
    let service = EnrollmentStatusService::new(&state.pool);
    let input = DropEnrollmentInput {
        enrollment_id: payload.enrollment_id,
        changed_by: payload.changed_by,
        reason: payload.reason,
        drop_date: parse_optional_date("drop_date", &payload.drop_date)?,
    };
    let result = service.drop_enrollment(&input).await?;
    Ok(Json(DropEnrollmentResponse { data: result.into() }))
}

async fn move_enrollment(
    State(state): State<ApiState>,
    Json(payload): Json<MoveEnrollmentRequest>,
) -> Result<Json<MoveEnrollmentResponse>, AppError> {
    let service = EnrollmentStatusService::new(&state.pool);
    let input = MoveWithinClubInput {
        enrollment_id: payload.enrollment_id,
        target_class_id: payload.target_class_id,
        changed_by: payload.changed_by,
    };
    let result = service.move_within_club(&input).await?;
    Ok(Json(MoveEnrollmentResponse { data: result.into() }))
}

async fn transfer_enrollment(
    State(state): State<ApiState>,
    Json(payload): Json<TransferEnrollmentRequest>,
) -> Result<Json<TransferEnrollmentResponse>, AppError> {
    let service = EnrollmentStatusService::new(&state.pool);
    let input = ClubTransferInput {
        source_enrollment_id: payload.source_enrollment_id,
        target_club_id: payload.target_club_id,
        target_weekday: payload.target_weekday,
        target_class_id: payload.target_class_id,
        changed_by: payload.changed_by,
        reason: payload.reason,
        drop_date: parse_optional_date("drop_date", &payload.drop_date)?,
    };
    let result = service.transfer_to_club(&input).await?;
    Ok(Json(TransferEnrollmentResponse { data: result.into() }))
}

impl From<DropEnrollmentResult> for DropEnrollmentDto {
    fn from(result: DropEnrollmentResult) -> Self {
        Self {
            enrollment_id: result.enrollment_id,
            from_status: result.from_status,
            to_status: result.to_status,
            drop_date: result.drop_date,
            waive_tuition_fee: result.waive_tuition_fee,
            tuition_grace_applied: result.tuition_grace_applied,
        }
    }
}

impl From<MoveWithinClubResult> for MoveEnrollmentDto {
    fn from(result: MoveWithinClubResult) -> Self {
        Self {
            enrollment_id: result.enrollment_id,
            previous_class_id: result.previous_class_id,
            new_class_id: result.new_class_id,
            status: result.status,
        }
    }
}

impl From<ClubTransferResult> for TransferEnrollmentDto {
    fn from(result: ClubTransferResult) -> Self {
        Self {
            from_enrollment_id: result.from_enrollment_id,
            to_enrollment_id: result.to_enrollment_id,
            drop_date: result.drop_date,
            waived_tuition_fee: result.waived_tuition_fee,
            tuition_grace_applied: result.tuition_grace_applied,
            carry_over_material_fee: result.carry_over_material_fee,
            new_material_fee_state: result.new_material_fee_state,
        }
    }
}

fn parse_optional_date(
    field: &str,
    raw: &Option<String>,
) -> Result<Option<NaiveDate>, AppError> {
    match raw {
        Some(value) => Ok(Some(parse_date(field, value)?)),
        None => Ok(None),
    }
}

fn parse_date(field: &str, value: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::Validation(format!("字段 `{}` 需使用 YYYY-MM-DD 格式", field)))
}
