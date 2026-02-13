use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::enrollment::MaterialFeeState;

/// 结算领域内的通用错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingError {
    message: String,
}

impl BillingError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// 结算批次类型：预览或正式。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BillingRunType {
    Preview,
    Final,
}

/// 结算批次执行状态。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BillingRunStatus {
    Pending,
    Running,
    Failed,
    Completed,
}

/// 费用项类型：课时费、材料费或调整。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BillingItemType {
    Tuition,
    Material,
    Adjustment,
}

/// `billing_runs` 的领域模型。
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingRun {
    pub id: Uuid,
    pub term_id: Uuid,
    pub run_type: BillingRunType,
    pub status: BillingRunStatus,
    pub triggered_by: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

/// `billing_items` 的领域模型。
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingItem {
    pub id: Uuid,
    pub billing_run_id: Uuid,
    pub enrollment_id: Uuid,
    pub item_type: BillingItemType,
    pub quantity: f64,
    pub unit_amount: f64,
    pub total_amount: f64,
    pub source_attendance: Option<u32>,
    pub policy_snapshot: BillingPolicySnapshot,
    pub note: Option<String>,
}

/// 记入 `policy_snapshot` 的核心字段。
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BillingPolicySnapshot {
    #[serde(default)]
    pub is_teacher_child: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teacher_discount_rate: Option<f64>,
    #[serde(default)]
    pub tuition_grace_applied: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waived_tuition_reason: Option<TuitionWaiverReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_charge_reason: Option<MaterialChargeReason>,
}

/// 课时费减免原因，写入 policy snapshot 便于审计。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TuitionWaiverReason {
    DropWithinGrace,
    ManualOverride,
    TeacherBenefit,
}

/// 材料费决策，用于记录是否重复收取。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MaterialChargeReason {
    ChargeOnce,
    AlreadyCharged,
    Refunded,
}

/// 教师子女优惠策略，`discount_rate=1.0` 表示全免，`0.5` 表示 5 折。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeacherDiscountPolicy {
    pub discount_rate: f64,
}

impl TeacherDiscountPolicy {
    pub fn validate(&self) -> Result<(), BillingError> {
        if !(0.0..=1.0).contains(&self.discount_rate) {
            return Err(BillingError::new("teacher_discount_rate 需在 0~1 范围内"));
        }
        Ok(())
    }
}

/// 课时费计算的输入参数。
#[derive(Debug, Clone)]
pub struct TuitionChargeInput {
    pub attendance_count: u32,
    pub price_per_session: f64,
    pub waive_tuition: bool,
    pub waive_reason: Option<TuitionWaiverReason>,
    pub is_teacher_child: bool,
    pub teacher_discount: Option<TeacherDiscountPolicy>,
}

/// 材料费计算输入。
#[derive(Debug, Clone, Copy)]
pub struct MaterialChargeInput {
    pub material_fee: f64,
    pub state: MaterialFeeState,
}

/// 课时费计算结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuitionCharge {
    pub charged_sessions: u32,
    pub waived_sessions: u32,
    pub unit_amount: f64,
    pub gross_amount: f64,
    pub discount_amount: f64,
    pub net_amount: f64,
    pub waiver_reason: Option<TuitionWaiverReason>,
    pub teacher_discount_applied: bool,
}

/// 材料费计算结果。
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialCharge {
    pub amount: f64,
    pub reason: MaterialChargeReason,
}

/// 前端展示所需的费用拆解。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBreakdown {
    pub enrollment_id: Uuid,
    pub student_id: Uuid,
    pub class_id: Uuid,
    pub material_fee: f64,
    pub lesson_fee: f64,
    pub discount_amount: f64,
    pub attendance_count: u32,
    pub charged_sessions: u32,
    pub waive_reason: Option<TuitionWaiverReason>,
    pub remarks: Option<String>,
}

/// 根据课堂出勤与策略计算课时费。
pub fn calculate_tuition_charge(input: &TuitionChargeInput) -> Result<TuitionCharge, BillingError> {
    if input.price_per_session < 0.0 {
        return Err(BillingError::new("price_per_session 不能为负数"));
    }

    if let Some(policy) = input.teacher_discount {
        policy.validate()?;
    }

    let charged_sessions = if input.waive_tuition {
        0
    } else {
        input.attendance_count
    };
    let waived_sessions = input.attendance_count.saturating_sub(charged_sessions);
    let unit_amount = input.price_per_session;
    let gross_amount = unit_amount * charged_sessions as f64;

    let mut discount_amount = 0.0;
    let mut teacher_discount_applied = false;
    if charged_sessions > 0 && input.is_teacher_child {
        if let Some(policy) = input.teacher_discount {
            discount_amount = gross_amount * policy.discount_rate;
            teacher_discount_applied = policy.discount_rate > 0.0;
        }
    }

    let waiver_reason = if input.waive_tuition {
        Some(
            input
                .waive_reason
                .unwrap_or(TuitionWaiverReason::ManualOverride),
        )
    } else {
        input.waive_reason
    };

    Ok(TuitionCharge {
        charged_sessions,
        waived_sessions,
        unit_amount,
        gross_amount,
        discount_amount,
        net_amount: (gross_amount - discount_amount).max(0.0),
        waiver_reason,
        teacher_discount_applied,
    })
}

/// 依据材料费状态判断是否需要收费。
pub fn evaluate_material_charge(
    input: &MaterialChargeInput,
) -> Result<MaterialCharge, BillingError> {
    if input.material_fee < 0.0 {
        return Err(BillingError::new("material_fee 不能为负数"));
    }

    let reason = match input.state {
        MaterialFeeState::Unset => MaterialChargeReason::ChargeOnce,
        MaterialFeeState::Charged => MaterialChargeReason::AlreadyCharged,
        MaterialFeeState::Refunded => MaterialChargeReason::Refunded,
    };

    let amount = if reason == MaterialChargeReason::ChargeOnce {
        input.material_fee
    } else {
        0.0
    };

    Ok(MaterialCharge { amount, reason })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuition_charge_without_discount() {
        let input = TuitionChargeInput {
            attendance_count: 8,
            price_per_session: 120.0,
            waive_tuition: false,
            waive_reason: None,
            is_teacher_child: false,
            teacher_discount: None,
        };
        let charge = calculate_tuition_charge(&input).unwrap();
        assert_eq!(charge.charged_sessions, 8);
        assert_eq!(charge.waived_sessions, 0);
        assert!((charge.gross_amount - 960.0).abs() < f64::EPSILON);
        assert!((charge.net_amount - 960.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tuition_charge_dropped_within_grace() {
        let input = TuitionChargeInput {
            attendance_count: 2,
            price_per_session: 100.0,
            waive_tuition: true,
            waive_reason: Some(TuitionWaiverReason::DropWithinGrace),
            is_teacher_child: false,
            teacher_discount: None,
        };
        let charge = calculate_tuition_charge(&input).unwrap();
        assert_eq!(charge.charged_sessions, 0);
        assert_eq!(charge.waived_sessions, 2);
        assert!((charge.net_amount - 0.0).abs() < f64::EPSILON);
        assert_eq!(
            charge.waiver_reason,
            Some(TuitionWaiverReason::DropWithinGrace)
        );
    }

    #[test]
    fn tuition_charge_teacher_child_discount() {
        let input = TuitionChargeInput {
            attendance_count: 6,
            price_per_session: 90.0,
            waive_tuition: false,
            waive_reason: None,
            is_teacher_child: true,
            teacher_discount: Some(TeacherDiscountPolicy { discount_rate: 0.5 }),
        };
        let charge = calculate_tuition_charge(&input).unwrap();
        assert_eq!(charge.charged_sessions, 6);
        assert!((charge.discount_amount - 270.0).abs() < f64::EPSILON);
        assert!((charge.net_amount - 270.0).abs() < f64::EPSILON);
        assert!(charge.teacher_discount_applied);
    }

    #[test]
    fn material_charge_only_when_unset() {
        let input = MaterialChargeInput {
            material_fee: 200.0,
            state: MaterialFeeState::Unset,
        };
        let charge = evaluate_material_charge(&input).unwrap();
        assert!((charge.amount - 200.0).abs() < f64::EPSILON);
        assert_eq!(charge.reason, MaterialChargeReason::ChargeOnce);

        let skipped = evaluate_material_charge(&MaterialChargeInput {
            material_fee: 200.0,
            state: MaterialFeeState::Charged,
        })
        .unwrap();
        assert_eq!(skipped.amount, 0.0);
        assert_eq!(skipped.reason, MaterialChargeReason::AlreadyCharged);
    }
}
