use chrono::{NaiveDate, Utc};
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};
use uuid::Uuid;

use crate::{
    db::DbPool,
    domain::{
        DropRuleContext, DropRuleDecision, EnrollmentStatus, EnrollmentTransition,
        MaterialFeeState, TransferKind, evaluate_material_fee_transition,
    },
    error::AppError,
};

type PgTx<'a> = Transaction<'a, Postgres>;

#[derive(Debug)]
pub struct EnrollmentStatusService<'a> {
    pool: &'a DbPool,
}

#[derive(Debug)]
pub struct DropEnrollmentInput {
    pub enrollment_id: Uuid,
    pub changed_by: String,
    pub reason: Option<String>,
    pub drop_date: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct DropEnrollmentResult {
    pub enrollment_id: Uuid,
    pub from_status: EnrollmentStatus,
    pub to_status: EnrollmentStatus,
    pub drop_date: NaiveDate,
    pub waive_tuition_fee: bool,
    pub tuition_grace_applied: bool,
}

#[derive(Debug)]
pub struct MoveWithinClubInput {
    pub enrollment_id: Uuid,
    pub target_class_id: Option<Uuid>,
    pub changed_by: String,
}

#[derive(Debug, Clone)]
pub struct MoveWithinClubResult {
    pub enrollment_id: Uuid,
    pub previous_class_id: Option<Uuid>,
    pub new_class_id: Option<Uuid>,
    pub status: EnrollmentStatus,
}

#[derive(Debug)]
pub struct ClubTransferInput {
    pub source_enrollment_id: Uuid,
    pub target_club_id: Uuid,
    pub target_weekday: u8,
    pub target_class_id: Option<Uuid>,
    pub changed_by: String,
    pub reason: Option<String>,
    pub drop_date: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct ClubTransferResult {
    pub from_enrollment_id: Uuid,
    pub to_enrollment_id: Uuid,
    pub drop_date: NaiveDate,
    pub waived_tuition_fee: bool,
    pub tuition_grace_applied: bool,
    pub carry_over_material_fee: bool,
    pub new_material_fee_state: MaterialFeeState,
}

#[derive(Debug, Clone)]
struct EnrollmentContext {
    id: Uuid,
    term_id: Uuid,
    campus_id: Uuid,
    student_id: Uuid,
    club_id: Uuid,
    class_id: Option<Uuid>,
    status: EnrollmentStatus,
    material_fee_state: MaterialFeeState,
    tuition_grace_applied: bool,
    grace_sessions: u16,
    requested_weekday: u8,
}

#[derive(Debug, Clone)]
struct ClassContext {
    term_id: Uuid,
    campus_id: Uuid,
    club_id: Uuid,
    weekday: u8,
}
impl<'a> EnrollmentStatusService<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn drop_enrollment(
        &self,
        input: &DropEnrollmentInput,
    ) -> Result<DropEnrollmentResult, AppError> {
        validate_actor(&input.changed_by)?;
        let reason = normalize_reason(input.reason.as_ref());
        let drop_date = input.drop_date.unwrap_or_else(|| Utc::now().date_naive());

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        let enrollment = self.fetch_enrollment(&mut tx, input.enrollment_id).await?;

        EnrollmentTransition::new(enrollment.status, EnrollmentStatus::Dropped)
            .validate()
            .map_err(|err| AppError::Validation(err.message().to_string()))?;

        ensure_editable_status(enrollment.status)?;

        let attendance_count = self
            .count_attendance_before(&mut tx, enrollment.id, drop_date)
            .await?;
        let drop_decision = evaluate_drop_decision(&enrollment, attendance_count);

        sqlx::query(
            r#"
                UPDATE enrollments
                SET status = 'DROPPED',
                    status_reason = $2,
                    drop_date = $3,
                    class_id = NULL,
                    tuition_grace_applied = $4
                WHERE id = $1
            "#,
        )
        .bind(enrollment.id)
        .bind(reason.as_deref())
        .bind(drop_date)
        .bind(drop_decision.resulting_grace_flag())
        .execute(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        self.insert_history(
            &mut tx,
            enrollment.id,
            Some(enrollment.status),
            EnrollmentStatus::Dropped,
            &input.changed_by,
            reason.as_deref(),
        )
        .await?;

        tx.commit()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        Ok(DropEnrollmentResult {
            enrollment_id: enrollment.id,
            from_status: enrollment.status,
            to_status: EnrollmentStatus::Dropped,
            drop_date,
            waive_tuition_fee: drop_decision.should_waive_tuition(),
            tuition_grace_applied: drop_decision.resulting_grace_flag(),
        })
    }
    pub async fn move_within_club(
        &self,
        input: &MoveWithinClubInput,
    ) -> Result<MoveWithinClubResult, AppError> {
        validate_actor(&input.changed_by)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;
        let enrollment = self.fetch_enrollment(&mut tx, input.enrollment_id).await?;

        ensure_editable_status(enrollment.status)?;

        if let Some(class_id) = input.target_class_id {
            let class_ctx = self.fetch_class(&mut tx, class_id).await?;
            if class_ctx.term_id != enrollment.term_id
                || class_ctx.campus_id != enrollment.campus_id
                || class_ctx.club_id != enrollment.club_id
            {
                return Err(AppError::Validation(
                    "目标班级不属于该学生当前报名上下文".into(),
                ));
            }
            if class_ctx.weekday != enrollment.requested_weekday {
                return Err(AppError::Validation(
                    "目标班级的星期与报名请求不一致".into(),
                ));
            }
        }

        sqlx::query(
            r#"
                UPDATE enrollments
                SET class_id = $2,
                    status = CASE
                        WHEN $2 IS NULL THEN 'PENDING'
                        ELSE 'ACTIVE'
                    END,
                    updated_at = now()
                WHERE id = $1
                  AND status IN ('PENDING','ACTIVE','TRANSFERRED_IN')
            "#,
        )
        .bind(enrollment.id)
        .bind(input.target_class_id)
        .execute(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        tx.commit()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        Ok(MoveWithinClubResult {
            enrollment_id: enrollment.id,
            previous_class_id: enrollment.class_id,
            new_class_id: input.target_class_id,
            status: if input.target_class_id.is_some() {
                EnrollmentStatus::Active
            } else {
                EnrollmentStatus::Pending
            },
        })
    }
    pub async fn transfer_to_club(
        &self,
        input: &ClubTransferInput,
    ) -> Result<ClubTransferResult, AppError> {
        validate_actor(&input.changed_by)?;
        validate_weekday(input.target_weekday)?;
        let reason = normalize_reason(input.reason.as_ref());
        let drop_date = input.drop_date.unwrap_or_else(|| Utc::now().date_naive());

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        let enrollment = self
            .fetch_enrollment(&mut tx, input.source_enrollment_id)
            .await?;

        if enrollment.club_id == input.target_club_id {
            return Err(AppError::Validation(
                "目标社团与当前相同，如需换班请使用换班功能".into(),
            ));
        }
        ensure_editable_status(enrollment.status)?;

        EnrollmentTransition::new(enrollment.status, EnrollmentStatus::TransferredOut)
            .validate()
            .map_err(|err| AppError::Validation(err.message().to_string()))?;

        self.ensure_club_exists(&mut tx, input.target_club_id)
            .await?;

        if let Some(class_id) = input.target_class_id {
            let class_ctx = self.fetch_class(&mut tx, class_id).await?;
            if class_ctx.term_id != enrollment.term_id
                || class_ctx.campus_id != enrollment.campus_id
            {
                return Err(AppError::Validation("目标班级不属于当前学期或校区".into()));
            }
            if class_ctx.club_id != input.target_club_id {
                return Err(AppError::Validation("目标班级不属于目标社团".into()));
            }
            if class_ctx.weekday != input.target_weekday {
                return Err(AppError::Validation(
                    "目标班级的星期与填写的星期不一致".into(),
                ));
            }
        }

        self.ensure_unique_target(
            &mut tx,
            enrollment.term_id,
            enrollment.campus_id,
            enrollment.student_id,
            input.target_club_id,
            input.target_weekday,
        )
        .await?;

        let attendance_count = self
            .count_attendance_before(&mut tx, enrollment.id, drop_date)
            .await?;
        let drop_decision = evaluate_drop_decision(&enrollment, attendance_count);

        sqlx::query(
            r#"
                UPDATE enrollments
                SET status = 'TRANSFERRED_OUT',
                    status_reason = $2,
                    drop_date = $3,
                    class_id = NULL,
                    tuition_grace_applied = $4
                WHERE id = $1
            "#,
        )
        .bind(enrollment.id)
        .bind(reason.as_deref())
        .bind(drop_date)
        .bind(drop_decision.resulting_grace_flag())
        .execute(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        self.insert_history(
            &mut tx,
            enrollment.id,
            Some(enrollment.status),
            EnrollmentStatus::TransferredOut,
            &input.changed_by,
            reason.as_deref(),
        )
        .await?;

        let fee_decision = evaluate_material_fee_transition(
            enrollment.material_fee_state,
            TransferKind::CrossClub,
        );

        let new_enrollment_id: Uuid = sqlx::query_scalar(
            r#"
                INSERT INTO enrollments (
                    term_id,
                    campus_id,
                    student_id,
                    club_id,
                    requested_weekday,
                    class_id,
                    status,
                    status_reason,
                    transferred_from_id,
                    material_fee_state,
                    tuition_grace_applied
                )
                VALUES ($1,$2,$3,$4,$5,$6,'TRANSFERRED_IN',$7,$8,$9,false)
                RETURNING id
            "#,
        )
        .bind(enrollment.term_id)
        .bind(enrollment.campus_id)
        .bind(enrollment.student_id)
        .bind(input.target_club_id)
        .bind(i16::from(input.target_weekday))
        .bind(input.target_class_id)
        .bind(reason.as_deref())
        .bind(enrollment.id)
        .bind(material_state_to_str(fee_decision.new_enrollment_state))
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| {
            if let sqlx::Error::Database(db_err) = &err {
                if db_err.code().as_deref() == Some("23505") {
                    return AppError::Conflict("目标社团已存在有效报名，无法转入".into());
                }
            }
            AppError::Database(err.to_string())
        })?;

        self.insert_history(
            &mut tx,
            new_enrollment_id,
            None,
            EnrollmentStatus::TransferredIn,
            &input.changed_by,
            reason.as_deref(),
        )
        .await?;

        tx.commit()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        Ok(ClubTransferResult {
            from_enrollment_id: enrollment.id,
            to_enrollment_id: new_enrollment_id,
            drop_date,
            waived_tuition_fee: drop_decision.should_waive_tuition(),
            tuition_grace_applied: drop_decision.resulting_grace_flag(),
            carry_over_material_fee: fee_decision.carry_over_previous_payment,
            new_material_fee_state: fee_decision.new_enrollment_state,
        })
    }
    async fn fetch_enrollment(
        &self,
        tx: &mut PgTx<'_>,
        enrollment_id: Uuid,
    ) -> Result<EnrollmentContext, AppError> {
        let row = sqlx::query(
            r#"
                SELECT e.id,
                       e.term_id,
                       e.campus_id,
                       e.student_id,
                       e.club_id,
                       e.class_id,
                       e.status,
                       e.material_fee_state,
                       e.tuition_grace_applied,
                       e.requested_weekday,
                       c.grace_sessions
                FROM enrollments e
                INNER JOIN clubs c ON c.id = e.club_id
                WHERE e.id = $1
                FOR UPDATE
            "#,
        )
        .bind(enrollment_id)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let row = row.ok_or_else(|| AppError::NotFound("未找到指定的报名记录".into()))?;
        map_enrollment_row(row)
    }

    async fn fetch_class(
        &self,
        tx: &mut PgTx<'_>,
        class_id: Uuid,
    ) -> Result<ClassContext, AppError> {
        let row = sqlx::query(
            r#"
                SELECT id, term_id, campus_id, club_id, weekday
                FROM classes
                WHERE id = $1
            "#,
        )
        .bind(class_id)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let row = row.ok_or_else(|| AppError::NotFound("目标班级不存在".into()))?;
        Ok(ClassContext {
            term_id: row
                .try_get("term_id")
                .map_err(|err| AppError::Database(err.to_string()))?,
            campus_id: row
                .try_get("campus_id")
                .map_err(|err| AppError::Database(err.to_string()))?,
            club_id: row
                .try_get("club_id")
                .map_err(|err| AppError::Database(err.to_string()))?,
            weekday: row
                .try_get::<i16, _>("weekday")
                .map_err(|err| AppError::Database(err.to_string()))? as u8,
        })
    }

    async fn ensure_unique_target(
        &self,
        tx: &mut PgTx<'_>,
        term_id: Uuid,
        campus_id: Uuid,
        student_id: Uuid,
        club_id: Uuid,
        weekday: u8,
    ) -> Result<(), AppError> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
                SELECT COUNT(*)
                FROM enrollments
                WHERE term_id = $1
                  AND campus_id = $2
                  AND student_id = $3
                  AND club_id = $4
                  AND requested_weekday = $5
                  AND status IN ('PENDING','ACTIVE','TRANSFERRED_IN')
            "#,
        )
        .bind(term_id)
        .bind(campus_id)
        .bind(student_id)
        .bind(club_id)
        .bind(i16::from(weekday))
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        if exists > 0 {
            return Err(AppError::Conflict(
                "该学生在目标社团与星期已存在有效报名".into(),
            ));
        }
        Ok(())
    }

    async fn ensure_club_exists(&self, tx: &mut PgTx<'_>, club_id: Uuid) -> Result<(), AppError> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
                SELECT COUNT(*)
                FROM clubs
                WHERE id = $1
            "#,
        )
        .bind(club_id)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        if exists == 0 {
            return Err(AppError::NotFound("目标社团不存在".into()));
        }
        Ok(())
    }

    async fn count_attendance_before(
        &self,
        tx: &mut PgTx<'_>,
        enrollment_id: Uuid,
        drop_date: NaiveDate,
    ) -> Result<i64, AppError> {
        let total = sqlx::query_scalar::<_, i64>(
            r#"
                SELECT COUNT(*)
                FROM attendance_records ar
                INNER JOIN class_meetings cm ON cm.id = ar.class_meeting_id
                WHERE ar.enrollment_id = $1
                  AND cm.meeting_date <= $2
            "#,
        )
        .bind(enrollment_id)
        .bind(drop_date)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        Ok(total)
    }

    async fn insert_history(
        &self,
        tx: &mut PgTx<'_>,
        enrollment_id: Uuid,
        from_status: Option<EnrollmentStatus>,
        to_status: EnrollmentStatus,
        changed_by: &str,
        note: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
                INSERT INTO enrollment_status_history (
                    enrollment_id,
                    from_status,
                    to_status,
                    changed_by,
                    note
                )
                VALUES ($1,$2,$3,$4,$5)
            "#,
        )
        .bind(enrollment_id)
        .bind(from_status.map(status_to_str))
        .bind(status_to_str(to_status))
        .bind(changed_by)
        .bind(note)
        .execute(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        Ok(())
    }
}
fn map_enrollment_row(row: PgRow) -> Result<EnrollmentContext, AppError> {
    let status_raw: String = row
        .try_get("status")
        .map_err(|err| AppError::Database(err.to_string()))?;
    let material_state_raw: String = row
        .try_get("material_fee_state")
        .map_err(|err| AppError::Database(err.to_string()))?;
    let grace_sessions: i16 = row
        .try_get("grace_sessions")
        .map_err(|err| AppError::Database(err.to_string()))?;

    Ok(EnrollmentContext {
        id: row
            .try_get("id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        term_id: row
            .try_get("term_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        campus_id: row
            .try_get("campus_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        student_id: row
            .try_get("student_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        club_id: row
            .try_get("club_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        class_id: row
            .try_get("class_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        status: parse_status(&status_raw),
        material_fee_state: parse_material_state(&material_state_raw),
        tuition_grace_applied: row
            .try_get("tuition_grace_applied")
            .map_err(|err| AppError::Database(err.to_string()))?,
        grace_sessions: normalize_grace_sessions(grace_sessions),
        requested_weekday: row
            .try_get::<i16, _>("requested_weekday")
            .map_err(|err| AppError::Database(err.to_string()))? as u8,
    })
}

fn evaluate_drop_decision(
    enrollment: &EnrollmentContext,
    attendance_count: i64,
) -> DropRuleDecision {
    let attended = clamp_to_u16(attendance_count);
    DropRuleContext {
        attended_sessions_before_drop: attended,
        grace_sessions: enrollment.grace_sessions,
        tuition_grace_already_applied: enrollment.tuition_grace_applied,
    }
    .evaluate()
}

fn parse_status(value: &str) -> EnrollmentStatus {
    match value {
        "ACTIVE" => EnrollmentStatus::Active,
        "DROPPED" => EnrollmentStatus::Dropped,
        "TRANSFERRED_OUT" => EnrollmentStatus::TransferredOut,
        "TRANSFERRED_IN" => EnrollmentStatus::TransferredIn,
        _ => EnrollmentStatus::Pending,
    }
}

fn status_to_str(value: EnrollmentStatus) -> &'static str {
    match value {
        EnrollmentStatus::Pending => "PENDING",
        EnrollmentStatus::Active => "ACTIVE",
        EnrollmentStatus::Dropped => "DROPPED",
        EnrollmentStatus::TransferredOut => "TRANSFERRED_OUT",
        EnrollmentStatus::TransferredIn => "TRANSFERRED_IN",
    }
}

pub(crate) fn parse_material_state(value: &str) -> MaterialFeeState {
    match value {
        "CHARGED" => MaterialFeeState::Charged,
        "REFUNDED" => MaterialFeeState::Refunded,
        _ => MaterialFeeState::Unset,
    }
}

pub(crate) fn material_state_to_str(state: MaterialFeeState) -> &'static str {
    match state {
        MaterialFeeState::Unset => "UNSET",
        MaterialFeeState::Charged => "CHARGED",
        MaterialFeeState::Refunded => "REFUNDED",
    }
}

pub(crate) fn normalize_reason(reason: Option<&String>) -> Option<String> {
    reason.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(crate) fn validate_actor(actor: &str) -> Result<(), AppError> {
    if actor.trim().is_empty() {
        Err(AppError::Validation("changed_by 不能为空".into()))
    } else {
        Ok(())
    }
}

pub(crate) fn ensure_editable_status(status: EnrollmentStatus) -> Result<(), AppError> {
    if matches!(
        status,
        EnrollmentStatus::Pending | EnrollmentStatus::Active | EnrollmentStatus::TransferredIn
    ) {
        Ok(())
    } else {
        Err(AppError::Validation("当前状态不可变更".into()))
    }
}

pub(crate) fn normalize_grace_sessions(value: i16) -> u16 {
    if value <= 0 { 0 } else { value as u16 }
}

pub(crate) fn clamp_to_u16(value: i64) -> u16 {
    if value <= 0 {
        0
    } else if value >= u16::MAX as i64 {
        u16::MAX
    } else {
        value as u16
    }
}

pub(crate) fn validate_weekday(weekday: u8) -> Result<(), AppError> {
    if (1..=7).contains(&weekday) {
        Ok(())
    } else {
        Err(AppError::Validation(
            "weekday 需在 1-7 之间（1=周一，7=周日）".into(),
        ))
    }
}
