pub mod attendance;
pub mod billing;
pub mod class_instance;
pub mod club;
pub mod enrollment;
pub mod enrollment_status;
pub mod student;

pub use attendance::{
    AttendanceExcelRow, AttendanceImportBatch, AttendanceImportRow, AttendanceRecord,
    AttendanceResult, AttendanceSessionKey, AttendanceStatus, AttendanceValidationError,
};
pub use billing::{
    BillingError, BillingItem, BillingItemType, BillingPolicySnapshot, BillingRun,
    BillingRunStatus, BillingRunType, FeeBreakdown, MaterialCharge, MaterialChargeInput,
    MaterialChargeReason, TeacherDiscountPolicy, TuitionCharge, TuitionChargeInput,
    TuitionWaiverReason, calculate_tuition_charge, evaluate_material_charge, requires_material_fee,
};
pub use class_instance::{ClassInstance, ClassStatus};
pub use club::Club;
pub use enrollment::{
    Enrollment, EnrollmentDraft, EnrollmentImportOutcome, EnrollmentImportStatus, EnrollmentStatus,
    MaterialFeeState,
};
pub use enrollment_status::{
    DropRuleContext, DropRuleDecision, EnrollmentStatusError, EnrollmentTransition,
    MaterialFeeDecision, TransferKind, evaluate_material_fee_transition,
};
pub use student::StudentProfile;
