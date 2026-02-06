use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 状态机枚举，覆盖 `enrollments.status` 列的全部取值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EnrollmentStatus {
    Pending,
    Active,
    Dropped,
    TransferredOut,
    TransferredIn,
}

/// 材料费收取状态，避免同一学生重复支付。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MaterialFeeState {
    Unset,
    Charged,
    Refunded,
}

impl Default for MaterialFeeState {
    fn default() -> Self {
        Self::Unset
    }
}

/// Excel 行的处理结果状态（供导入服务返回给前端/日志）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EnrollmentImportStatus {
    Pending,
    Created,
    Skipped,
    Failed,
}

/// 与 `enrollments` 表一一对应的领域模型。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrollment {
    pub id: Uuid,
    pub term_id: Uuid,
    pub campus_id: Uuid,
    pub student_id: Uuid,
    pub club_id: Uuid,
    pub requested_weekday: u8,
    pub class_id: Option<Uuid>,
    pub import_job_id: Option<Uuid>,
    pub status: EnrollmentStatus,
    pub status_reason: Option<String>,
    pub drop_date: Option<NaiveDate>,
    pub transferred_from_id: Option<Uuid>,
    pub material_fee_state: MaterialFeeState,
    pub tuition_grace_applied: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 由 Excel 解析得到的一条“待建”报名草稿记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentDraft {
    pub term_id: Uuid,
    pub homeroom_display_name: String,
    pub student_full_name: String,
    pub student_code: Option<String>,
    pub requested_weekday: u8,
    /// Excel 的原始社团文本（可能是名称或编码），由导入服务解析为 club_id。
    pub club_lookup_value: String,
    pub source_row: u32,
    /// Excel 原始“班级+姓名”字段，便于兜底匹配或错误提示。
    pub raw_identifier: String,
}

/// 导入单行处理后的反馈，便于 API 返回详细错误。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentImportOutcome {
    /// 对应的 Excel 行（1-based）。
    pub source_row: u32,
    pub draft: Option<EnrollmentDraft>,
    pub status: EnrollmentImportStatus,
    pub enrollment_id: Option<Uuid>,
    pub message: Option<String>,
}
