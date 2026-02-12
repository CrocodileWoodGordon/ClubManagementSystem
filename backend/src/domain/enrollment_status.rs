use super::enrollment::{EnrollmentStatus, MaterialFeeState};

/// 状态切换异常，避免服务层重复描述字符串。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrollmentStatusError {
    message: String,
}

impl EnrollmentStatusError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// 状态流转（from → to）的描述体，负责统一校验逻辑。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnrollmentTransition {
    pub from: EnrollmentStatus,
    pub to: EnrollmentStatus,
}

impl EnrollmentTransition {
    pub fn new(from: EnrollmentStatus, to: EnrollmentStatus) -> Self {
        Self { from, to }
    }

    pub fn validate(&self) -> Result<(), EnrollmentStatusError> {
        if self.from == self.to {
            return Ok(());
        }

        if is_allowed_transition(self.from, self.to) {
            Ok(())
        } else {
            Err(EnrollmentStatusError::new(format!(
                "状态 {:?} 不允许切换为 {:?}",
                self.from, self.to
            )))
        }
    }
}

fn is_allowed_transition(from: EnrollmentStatus, to: EnrollmentStatus) -> bool {
    matches!(
        (from, to),
        // 报名 → 分班 / 退课
        (EnrollmentStatus::Pending, EnrollmentStatus::Active)
            | (EnrollmentStatus::Pending, EnrollmentStatus::Dropped)
            | (EnrollmentStatus::Pending, EnrollmentStatus::TransferredIn)
            // 正常在读 → 退课 / 转出
            | (EnrollmentStatus::Active, EnrollmentStatus::Dropped)
            | (EnrollmentStatus::Active, EnrollmentStatus::TransferredOut)
            // 退课后允许恢复
            | (EnrollmentStatus::Dropped, EnrollmentStatus::Active)
            // 转出的旧记录最终视同退课
            | (EnrollmentStatus::TransferredOut, EnrollmentStatus::Dropped)
            // 新记录转入后需重新激活或直接退课
            | (EnrollmentStatus::TransferredIn, EnrollmentStatus::Active)
            | (EnrollmentStatus::TransferredIn, EnrollmentStatus::Dropped)
    )
}

/// 换课类型：区分“同社团换班”与“跨社团转入”。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    SameClub,
    CrossClub,
}

/// 材料费决策结果，供服务层据此决定是否要重新收费。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialFeeDecision {
    pub carry_over_previous_payment: bool,
    pub new_enrollment_state: MaterialFeeState,
}

impl MaterialFeeDecision {
    pub fn requires_new_charge(&self) -> bool {
        !self.carry_over_previous_payment
            || matches!(self.new_enrollment_state, MaterialFeeState::Unset)
    }
}

pub fn evaluate_material_fee_transition(
    original_state: MaterialFeeState,
    transfer_kind: TransferKind,
) -> MaterialFeeDecision {
    match transfer_kind {
        TransferKind::SameClub => MaterialFeeDecision {
            carry_over_previous_payment: true,
            new_enrollment_state: original_state,
        },
        TransferKind::CrossClub => MaterialFeeDecision {
            carry_over_previous_payment: false,
            new_enrollment_state: MaterialFeeState::Unset,
        },
    }
}

/// 退课免课时费的判定上下文。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropRuleContext {
    pub attended_sessions_before_drop: u16,
    pub grace_sessions: u16,
    pub tuition_grace_already_applied: bool,
}

impl DropRuleContext {
    pub fn evaluate(self) -> DropRuleDecision {
        if self.tuition_grace_already_applied {
            return DropRuleDecision {
                waive_tuition_fee: false,
                tuition_grace_applied: true,
            };
        }

        if self.grace_sessions == 0 {
            return DropRuleDecision {
                waive_tuition_fee: false,
                tuition_grace_applied: false,
            };
        }

        if u32::from(self.attended_sessions_before_drop) <= u32::from(self.grace_sessions) {
            DropRuleDecision {
                waive_tuition_fee: true,
                tuition_grace_applied: true,
            }
        } else {
            DropRuleDecision {
                waive_tuition_fee: false,
                tuition_grace_applied: false,
            }
        }
    }
}

/// 退课判定结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropRuleDecision {
    pub waive_tuition_fee: bool,
    pub tuition_grace_applied: bool,
}

impl DropRuleDecision {
    pub fn should_waive_tuition(&self) -> bool {
        self.waive_tuition_fee
    }

    pub fn resulting_grace_flag(&self) -> bool {
        self.tuition_grace_applied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_basic_transitions() {
        assert!(
            EnrollmentTransition::new(EnrollmentStatus::Pending, EnrollmentStatus::Active)
                .validate()
                .is_ok()
        );
        assert!(
            EnrollmentTransition::new(EnrollmentStatus::Active, EnrollmentStatus::TransferredOut)
                .validate()
                .is_ok()
        );
        assert!(
            EnrollmentTransition::new(EnrollmentStatus::TransferredIn, EnrollmentStatus::Active)
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn reject_illegal_transitions() {
        let err =
            EnrollmentTransition::new(EnrollmentStatus::TransferredOut, EnrollmentStatus::Active)
                .validate()
                .unwrap_err();
        assert!(err.message().contains("不允许"));
    }

    #[test]
    fn reuse_material_fee_for_same_club() {
        let decision =
            evaluate_material_fee_transition(MaterialFeeState::Charged, TransferKind::SameClub);
        assert!(decision.carry_over_previous_payment);
        assert_eq!(decision.new_enrollment_state, MaterialFeeState::Charged);
        assert!(!decision.requires_new_charge());
    }

    #[test]
    fn reset_material_fee_for_cross_club() {
        let decision =
            evaluate_material_fee_transition(MaterialFeeState::Charged, TransferKind::CrossClub);
        assert!(!decision.carry_over_previous_payment);
        assert_eq!(decision.new_enrollment_state, MaterialFeeState::Unset);
        assert!(decision.requires_new_charge());
    }

    #[test]
    fn waive_tuition_within_grace() {
        let ctx = DropRuleContext {
            attended_sessions_before_drop: 2,
            grace_sessions: 3,
            tuition_grace_already_applied: false,
        };
        let decision = ctx.evaluate();
        assert!(decision.should_waive_tuition());
        assert!(decision.resulting_grace_flag());
    }

    #[test]
    fn charge_tuition_after_grace_consumed() {
        let ctx = DropRuleContext {
            attended_sessions_before_drop: 5,
            grace_sessions: 3,
            tuition_grace_already_applied: false,
        };
        let decision = ctx.evaluate();
        assert!(!decision.should_waive_tuition());
        assert!(!decision.resulting_grace_flag());
    }

    #[test]
    fn keep_existing_flag_when_already_applied() {
        let ctx = DropRuleContext {
            attended_sessions_before_drop: 1,
            grace_sessions: 3,
            tuition_grace_already_applied: true,
        };
        let decision = ctx.evaluate();
        assert!(!decision.should_waive_tuition());
        assert!(decision.resulting_grace_flag());
    }
}
