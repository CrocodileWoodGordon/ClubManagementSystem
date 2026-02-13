use sqlx::{FromRow, types::BigDecimal};
use uuid::Uuid;

use crate::{
    db::DbPool,
    domain::{
        EnrollmentStatus, FeeBreakdown, MaterialChargeInput, MaterialChargeReason,
        MaterialFeeState, TeacherDiscountPolicy, TuitionChargeInput, TuitionWaiverReason,
        calculate_tuition_charge, evaluate_material_charge,
    },
    error::AppError,
};

const TEACHER_CHILD_DISCOUNT_RATE: f64 = 1.0;

#[derive(Debug)]
pub struct BillingService<'a> {
    pool: &'a DbPool,
}

#[derive(Debug, Clone)]
pub struct FeeBreakdownDetail {
    pub breakdown: FeeBreakdown,
    pub club_id: Uuid,
    pub club_name: String,
    pub class_code: Option<String>,
}

impl<'a> BillingService<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// 汇总指定班级内所有报名记录的费用明细。
    pub async fn preview_by_class(&self, class_id: Uuid) -> Result<Vec<FeeBreakdown>, AppError> {
        let ctx = self.fetch_class_context(class_id).await?;
        let rows = sqlx::query_as::<_, BillingSourceRow>(
            r#"
                SELECT e.id AS enrollment_id,
                       e.student_id,
                       COALESCE(e.class_id, $1) AS resolved_class_id,
                       e.club_id,
                       c.name AS club_name,
                       cls.class_code,
                       e.status,
                       e.status_reason,
                       e.material_fee_state,
                       e.tuition_grace_applied,
                       s.is_teacher_child,
                       c.grace_sessions,
                       COALESCE(ct.price_per_session, c.price_per_session) AS price_per_session,
                       COALESCE(ct.material_fee, c.material_fee) AS material_fee,
                       COALESCE(att.present_count, 0)::bigint AS attendance_count
                FROM enrollments e
                INNER JOIN students s ON s.id = e.student_id
                INNER JOIN clubs c ON c.id = e.club_id
                LEFT JOIN classes cls ON cls.id = COALESCE(e.class_id, $1)
                LEFT JOIN club_terms ct
                       ON ct.term_id = e.term_id
                      AND ct.campus_id = e.campus_id
                      AND ct.club_id = e.club_id
                LEFT JOIN (
                    SELECT ar.enrollment_id,
                           COUNT(*) FILTER (WHERE ar.status = 'PRESENT')::bigint AS present_count
                    FROM attendance_records ar
                    INNER JOIN class_meetings cm ON cm.id = ar.class_meeting_id
                    WHERE cm.class_id = $1
                    GROUP BY ar.enrollment_id
                ) att ON att.enrollment_id = e.id
                WHERE e.term_id = $2
                  AND e.campus_id = $3
                  AND e.club_id = $4
                  AND e.status IN ('ACTIVE','DROPPED','TRANSFERRED_OUT','TRANSFERRED_IN')
                  AND (e.class_id = $1 OR att.present_count IS NOT NULL)
                ORDER BY e.created_at ASC
            "#,
        )
        .bind(class_id)
        .bind(ctx.term_id)
        .bind(ctx.campus_id)
        .bind(ctx.club_id)
        .fetch_all(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        rows.into_iter().map(build_breakdown).collect()
    }

    /// 按学生预览当前学期的费用，输出分社团/班级的行。
    pub async fn preview_by_student(
        &self,
        student_id: Uuid,
    ) -> Result<Vec<FeeBreakdown>, AppError> {
        let term_id = self.resolve_active_term_id().await?;
        let rows = sqlx::query_as::<_, BillingSourceRow>(
            r#"
                SELECT e.id AS enrollment_id,
                       e.student_id,
                       COALESCE(e.class_id, fallback.class_id) AS resolved_class_id,
                       e.club_id,
                       c.name AS club_name,
                       cls.class_code,
                       e.status,
                       e.status_reason,
                       e.material_fee_state,
                       e.tuition_grace_applied,
                       s.is_teacher_child,
                       c.grace_sessions,
                       COALESCE(ct.price_per_session, c.price_per_session) AS price_per_session,
                       COALESCE(ct.material_fee, c.material_fee) AS material_fee,
                       COALESCE(att.present_count, 0)::bigint AS attendance_count
                FROM enrollments e
                INNER JOIN students s ON s.id = e.student_id
                INNER JOIN clubs c ON c.id = e.club_id
                LEFT JOIN classes cls ON cls.id = COALESCE(e.class_id, fallback.class_id)
                LEFT JOIN club_terms ct
                       ON ct.term_id = e.term_id
                      AND ct.campus_id = e.campus_id
                      AND ct.club_id = e.club_id
                LEFT JOIN (
                    SELECT ar.enrollment_id,
                           COUNT(*) FILTER (WHERE ar.status = 'PRESENT')::bigint AS present_count
                    FROM attendance_records ar
                    GROUP BY ar.enrollment_id
                ) att ON att.enrollment_id = e.id
                LEFT JOIN LATERAL (
                    SELECT cm.class_id
                    FROM attendance_records ar
                    INNER JOIN class_meetings cm ON cm.id = ar.class_meeting_id
                    WHERE ar.enrollment_id = e.id
                    GROUP BY cm.class_id
                    ORDER BY COUNT(*) DESC
                    LIMIT 1
                ) fallback ON true
                WHERE e.student_id = $1
                  AND e.term_id = $2
                  AND e.status IN ('ACTIVE','DROPPED','TRANSFERRED_OUT','TRANSFERRED_IN')
                  AND COALESCE(e.class_id, fallback.class_id) IS NOT NULL
                ORDER BY e.created_at ASC
            "#,
        )
        .bind(student_id)
        .bind(term_id)
        .fetch_all(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        rows.into_iter().map(build_breakdown).collect()
    }

    pub async fn preview_by_students_bulk(
        &self,
        student_ids: Vec<Uuid>,
        term_id: Uuid,
    ) -> Result<Vec<FeeBreakdownDetail>, AppError> {
        if student_ids.is_empty() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query_as::<_, BillingSourceRow>(
            r#"
                SELECT e.id AS enrollment_id,
                       e.student_id,
                       COALESCE(e.class_id, fallback.class_id) AS resolved_class_id,
                       e.club_id,
                       c.name AS club_name,
                       cls.class_code,
                       e.status,
                       e.status_reason,
                       e.material_fee_state,
                       e.tuition_grace_applied,
                       s.is_teacher_child,
                       c.grace_sessions,
                       COALESCE(ct.price_per_session, c.price_per_session) AS price_per_session,
                       COALESCE(ct.material_fee, c.material_fee) AS material_fee,
                       COALESCE(att.present_count, 0)::bigint AS attendance_count
                FROM enrollments e
                INNER JOIN students s ON s.id = e.student_id
                INNER JOIN clubs c ON c.id = e.club_id
                LEFT JOIN club_terms ct
                       ON ct.term_id = e.term_id
                      AND ct.campus_id = e.campus_id
                      AND ct.club_id = e.club_id
                LEFT JOIN (
                    SELECT ar.enrollment_id,
                           COUNT(*) FILTER (WHERE ar.status = 'PRESENT')::bigint AS present_count
                    FROM attendance_records ar
                    GROUP BY ar.enrollment_id
                ) att ON att.enrollment_id = e.id
                LEFT JOIN LATERAL (
                    SELECT cm.class_id
                    FROM attendance_records ar
                    INNER JOIN class_meetings cm ON cm.id = ar.class_meeting_id
                    WHERE ar.enrollment_id = e.id
                    GROUP BY cm.class_id
                    ORDER BY COUNT(*) DESC
                    LIMIT 1
                ) fallback ON true
                LEFT JOIN classes cls ON cls.id = COALESCE(e.class_id, fallback.class_id)
                WHERE e.student_id = ANY($1::uuid[])
                  AND e.term_id = $2
                  AND e.status IN ('ACTIVE','DROPPED','TRANSFERRED_OUT','TRANSFERRED_IN')
                  AND COALESCE(e.class_id, fallback.class_id) IS NOT NULL
                ORDER BY s.full_name ASC, e.created_at ASC
            "#,
        )
        .bind(&student_ids)
        .bind(term_id)
        .fetch_all(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        rows.into_iter()
            .map(|row| {
                let club_id = row.club_id;
                let club_name = row.club_name.clone();
                let class_code = row.class_code.clone();
                build_breakdown(row).map(|breakdown| FeeBreakdownDetail {
                    breakdown,
                    club_id,
                    club_name,
                    class_code,
                })
            })
            .collect()
    }

    async fn fetch_class_context(&self, class_id: Uuid) -> Result<ClassContext, AppError> {
        sqlx::query_as::<_, ClassContext>(
            r#"
                SELECT term_id, campus_id, club_id
                FROM classes
                WHERE id = $1
            "#,
        )
        .bind(class_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?
        .ok_or_else(|| AppError::NotFound("未找到指定班级".into()))
    }

    async fn resolve_active_term_id(&self) -> Result<Uuid, AppError> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
                SELECT id
                FROM terms
                WHERE is_active = true
                ORDER BY enrollment_start DESC
                LIMIT 1
            "#,
        )
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?
        .ok_or_else(|| AppError::Validation("未找到激活学期，请先设置当前学期".into()))
    }
}

#[derive(Debug, FromRow)]
struct ClassContext {
    term_id: Uuid,
    campus_id: Uuid,
    club_id: Uuid,
}

#[derive(Debug, FromRow)]
struct BillingSourceRow {
    enrollment_id: Uuid,
    student_id: Uuid,
    resolved_class_id: Uuid,
    club_id: Uuid,
    club_name: String,
    class_code: Option<String>,
    status: String,
    status_reason: Option<String>,
    material_fee_state: String,
    tuition_grace_applied: bool,
    is_teacher_child: bool,
    grace_sessions: i16,
    price_per_session: BigDecimal,
    material_fee: BigDecimal,
    attendance_count: Option<i64>,
}

fn build_breakdown(row: BillingSourceRow) -> Result<FeeBreakdown, AppError> {
    let status = map_status(&row.status);
    let attendance = clamp_to_u32(row.attendance_count.unwrap_or(0));
    let grace_limit = normalize_grace_sessions(row.grace_sessions);
    let attendance_for_grace = clamp_to_u16(attendance);
    let drop_status = matches!(
        status,
        EnrollmentStatus::Dropped | EnrollmentStatus::TransferredOut
    );
    let waive_due_to_grace = drop_status
        && (row.tuition_grace_applied || (grace_limit > 0 && attendance_for_grace <= grace_limit));

    let mut waive_reason = if waive_due_to_grace {
        Some(TuitionWaiverReason::DropWithinGrace)
    } else {
        None
    };

    let teacher_discount = if row.is_teacher_child && !waive_due_to_grace {
        if waive_reason.is_none() {
            waive_reason = Some(TuitionWaiverReason::TeacherBenefit);
        }
        Some(TeacherDiscountPolicy {
            discount_rate: TEACHER_CHILD_DISCOUNT_RATE,
        })
    } else {
        None
    };

    let tuition = calculate_tuition_charge(&TuitionChargeInput {
        attendance_count: attendance,
        price_per_session: decimal_to_f64(row.price_per_session),
        waive_tuition: waive_due_to_grace,
        waive_reason,
        is_teacher_child: row.is_teacher_child,
        teacher_discount,
    })
    .map_err(map_billing_error)?;

    let material_charge = evaluate_material_charge(&MaterialChargeInput {
        material_fee: decimal_to_f64(row.material_fee),
        state: parse_material_state(&row.material_fee_state),
    })
    .map_err(map_billing_error)?;

    Ok(FeeBreakdown {
        enrollment_id: row.enrollment_id,
        student_id: row.student_id,
        class_id: row.resolved_class_id,
        material_fee: material_charge.amount,
        lesson_fee: tuition.net_amount,
        discount_amount: tuition.discount_amount,
        attendance_count: attendance,
        charged_sessions: tuition.charged_sessions,
        waive_reason: tuition.waiver_reason,
        remarks: compose_remarks(row.status_reason, material_charge.reason),
    })
}

fn map_status(raw: &str) -> EnrollmentStatus {
    match raw {
        "ACTIVE" => EnrollmentStatus::Active,
        "DROPPED" => EnrollmentStatus::Dropped,
        "TRANSFERRED_OUT" => EnrollmentStatus::TransferredOut,
        "TRANSFERRED_IN" => EnrollmentStatus::TransferredIn,
        _ => EnrollmentStatus::Pending,
    }
}

fn parse_material_state(raw: &str) -> MaterialFeeState {
    match raw {
        "CHARGED" => MaterialFeeState::Charged,
        "REFUNDED" => MaterialFeeState::Refunded,
        _ => MaterialFeeState::Unset,
    }
}

fn compose_remarks(base: Option<String>, reason: MaterialChargeReason) -> Option<String> {
    match reason {
        MaterialChargeReason::ChargeOnce => base,
        MaterialChargeReason::AlreadyCharged => Some(match base {
            Some(text) if !text.trim().is_empty() => format!("{text}；材料费已收"),
            _ => "材料费已收".to_string(),
        }),
        MaterialChargeReason::Refunded => Some(match base {
            Some(text) if !text.trim().is_empty() => format!("{text}；材料费已退"),
            _ => "材料费已退".to_string(),
        }),
    }
}

fn decimal_to_f64(value: BigDecimal) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(0.0)
}

fn clamp_to_u32(value: i64) -> u32 {
    if value <= 0 {
        0
    } else if value >= u32::MAX as i64 {
        u32::MAX
    } else {
        value as u32
    }
}

fn clamp_to_u16(value: u32) -> u16 {
    if value > u16::MAX as u32 {
        u16::MAX
    } else {
        value as u16
    }
}

fn normalize_grace_sessions(value: i16) -> u16 {
    if value <= 0 { 0 } else { value as u16 }
}

fn map_billing_error(err: crate::domain::BillingError) -> AppError {
    AppError::Validation(err.message().to_string())
}
