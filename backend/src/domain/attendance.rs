use std::{convert::TryFrom, fmt};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::AttendanceRecordRow;

pub type AttendanceResult<T> = Result<T, AttendanceValidationError>;

/// 领域层对考勤相关错误的描述，供服务/导入流程统一使用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttendanceValidationError {
    InvalidStatus { row: Option<u32>, raw: String },
    InvalidMinutes { row: u32, raw: String },
    MissingIdentifier { row: u32 },
    EmptyBatch,
    InvalidSessionNumber { value: i32 },
}

impl fmt::Display for AttendanceValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttendanceValidationError::InvalidStatus { row, raw } => {
                if let Some(row_number) = row {
                    write!(f, "第 {} 行的考勤状态无效: {}", row_number, raw)
                } else {
                    write!(f, "考勤状态无效: {}", raw)
                }
            }
            AttendanceValidationError::InvalidMinutes { row, raw } => {
                write!(f, "第 {} 行的考勤时长无效: {}", row, raw)
            }
            AttendanceValidationError::MissingIdentifier { row } => {
                write!(f, "第 {} 行缺少学生标识", row)
            }
            AttendanceValidationError::EmptyBatch => {
                write!(f, "考勤导入内容为空")
            }
            AttendanceValidationError::InvalidSessionNumber { value } => {
                write!(f, "课次编号必须大于 0，当前值 {}", value)
            }
        }
    }
}

impl std::error::Error for AttendanceValidationError {}

/// Attendance status aligned with attendance_records.status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttendanceStatus {
    Present,
    Absent,
    Excused,
    Leave,
}

impl AttendanceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AttendanceStatus::Present => "PRESENT",
            AttendanceStatus::Absent => "ABSENT",
            AttendanceStatus::Excused => "EXCUSED",
            AttendanceStatus::Leave => "LEAVE",
        }
    }

    pub fn from_excel(value: &str, row: u32) -> AttendanceResult<Self> {
        Self::try_from(value).map_err(|_| AttendanceValidationError::InvalidStatus {
            row: Some(row),
            raw: value.trim().to_string(),
        })
    }

    fn parse_label(value: &str) -> Option<Self> {
        let normalized = value.trim().to_uppercase();
        match normalized.as_str() {
            "" => Some(AttendanceStatus::Present),
            "P" | "PRESENT" | "出勤" | "正常" => Some(AttendanceStatus::Present),
            "A" | "ABSENT" | "缺勤" => Some(AttendanceStatus::Absent),
            "E" | "EXCUSED" | "SICK" | "病假" => Some(AttendanceStatus::Excused),
            "L" | "LEAVE" | "请假" | "事假" => Some(AttendanceStatus::Leave),
            _ => None,
        }
    }
}

impl TryFrom<&str> for AttendanceStatus {
    type Error = AttendanceValidationError;

    fn try_from(value: &str) -> AttendanceResult<Self> {
        AttendanceStatus::parse_label(value).ok_or_else(|| {
            AttendanceValidationError::InvalidStatus {
                row: None,
                raw: value.trim().to_string(),
            }
        })
    }
}

/// 标识具体的班级课次（class + date + 第几节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttendanceSessionKey {
    pub class_id: Uuid,
    pub meeting_date: NaiveDate,
    pub session_number: u16,
}

impl AttendanceSessionKey {
    pub fn new(
        class_id: Uuid,
        meeting_date: NaiveDate,
        session_number: u16,
    ) -> AttendanceResult<Self> {
        if session_number == 0 {
            return Err(AttendanceValidationError::InvalidSessionNumber { value: 0 });
        }
        Ok(Self {
            class_id,
            meeting_date,
            session_number,
        })
    }
}

/// 映射 attendance_records 表。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceRecord {
    pub id: Uuid,
    pub class_meeting_id: Uuid,
    pub enrollment_id: Uuid,
    pub status: AttendanceStatus,
    pub minutes_attended: Option<i32>,
    pub recorded_by: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

impl TryFrom<AttendanceRecordRow> for AttendanceRecord {
    type Error = AttendanceValidationError;

    fn try_from(row: AttendanceRecordRow) -> AttendanceResult<Self> {
        Ok(Self {
            id: row.id,
            class_meeting_id: row.class_meeting_id,
            enrollment_id: row.enrollment_id,
            status: AttendanceStatus::try_from(row.status.as_str())?,
            minutes_attended: row.minutes_attended,
            recorded_by: row.recorded_by.filter(|value| !value.trim().is_empty()),
            recorded_at: row.recorded_at,
        })
    }
}

/// Excel 行解析后的原始载体，携带尚未校验的字符串值。
#[derive(Debug, Clone)]
pub struct AttendanceExcelRow {
    pub source_row: u32,
    pub student_identifier: String,
    pub status_text: String,
    pub minutes_value: Option<String>,
    pub note: Option<String>,
}

/// 经过解析与校验后的行，等待与 enrollment/class meeting 对应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceImportRow {
    pub source_row: u32,
    pub student_identifier: String,
    pub enrollment_id: Option<Uuid>,
    pub status: AttendanceStatus,
    pub minutes_attended: Option<i32>,
    pub note: Option<String>,
}

impl AttendanceImportRow {
    pub fn with_enrollment(mut self, enrollment_id: Uuid) -> Self {
        self.enrollment_id = Some(enrollment_id);
        self
    }

    pub fn identifier_key(&self) -> String {
        normalize_identifier(&self.student_identifier)
    }

    pub fn validate_identifier(&self) -> AttendanceResult<()> {
        if self.student_identifier.trim().is_empty() {
            Err(AttendanceValidationError::MissingIdentifier {
                row: self.source_row,
            })
        } else {
            Ok(())
        }
    }
}

impl TryFrom<AttendanceExcelRow> for AttendanceImportRow {
    type Error = AttendanceValidationError;

    fn try_from(row: AttendanceExcelRow) -> AttendanceResult<Self> {
        let AttendanceExcelRow {
            source_row,
            student_identifier,
            status_text,
            minutes_value,
            note,
        } = row;

        let identifier = student_identifier.trim().to_string();
        if identifier.is_empty() {
            return Err(AttendanceValidationError::MissingIdentifier { row: source_row });
        }

        Ok(Self {
            source_row,
            student_identifier: identifier,
            enrollment_id: None,
            status: AttendanceStatus::from_excel(&status_text, source_row)?,
            minutes_attended: parse_minutes(minutes_value, source_row)?,
            note: normalize_note(note),
        })
    }
}

/// Excel 批量导入的聚合结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceImportBatch {
    pub batch_id: Uuid,
    pub import_job_id: Option<Uuid>,
    pub class_meeting_id: Uuid,
    pub session: AttendanceSessionKey,
    pub recorded_by: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub rows: Vec<AttendanceImportRow>,
}

impl AttendanceImportBatch {
    pub fn new(
        session: AttendanceSessionKey,
        class_meeting_id: Uuid,
        recorded_by: Option<String>,
        rows: Vec<AttendanceImportRow>,
        import_job_id: Option<Uuid>,
    ) -> AttendanceResult<Self> {
        if rows.is_empty() {
            return Err(AttendanceValidationError::EmptyBatch);
        }

        for row in &rows {
            row.validate_identifier()?;
        }

        Ok(Self {
            batch_id: Uuid::new_v4(),
            import_job_id,
            class_meeting_id,
            session,
            recorded_by: normalize_note(recorded_by),
            submitted_at: Utc::now(),
            rows,
        })
    }
}

fn normalize_identifier(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_note(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn parse_minutes(raw: Option<String>, row: u32) -> AttendanceResult<Option<i32>> {
    match raw {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            let parsed: i32 =
                trimmed
                    .parse()
                    .map_err(|_| AttendanceValidationError::InvalidMinutes {
                        row,
                        raw: trimmed.to_string(),
                    })?;
            if parsed < 0 {
                return Err(AttendanceValidationError::InvalidMinutes {
                    row,
                    raw: trimmed.to_string(),
                });
            }
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::AttendanceRecordRow;
    use chrono::TimeZone;

    #[test]
    fn status_parsing_accepts_aliases() {
        assert_eq!(
            AttendanceStatus::try_from("present").unwrap(),
            AttendanceStatus::Present
        );
        assert_eq!(
            AttendanceStatus::try_from("缺勤").unwrap(),
            AttendanceStatus::Absent
        );
        assert_eq!(
            AttendanceStatus::from_excel("请假", 5).unwrap(),
            AttendanceStatus::Leave
        );
    }

    #[test]
    fn session_key_validates_number() {
        let class_id = Uuid::new_v4();
        let meeting_date = NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
        assert!(AttendanceSessionKey::new(class_id, meeting_date, 1).is_ok());
        assert!(matches!(
            AttendanceSessionKey::new(class_id, meeting_date, 0),
            Err(AttendanceValidationError::InvalidSessionNumber { .. })
        ));
    }

    #[test]
    fn record_try_from_row() {
        let row = AttendanceRecordRow {
            id: Uuid::new_v4(),
            class_meeting_id: Uuid::new_v4(),
            enrollment_id: Uuid::new_v4(),
            status: "PRESENT".into(),
            minutes_attended: Some(90),
            recorded_by: Some("Alice".into()),
            recorded_at: Utc.with_ymd_and_hms(2026, 2, 1, 2, 0, 0).unwrap(),
        };

        let record = AttendanceRecord::try_from(row).unwrap();
        assert_eq!(record.status, AttendanceStatus::Present);
        assert_eq!(record.minutes_attended, Some(90));
    }

    #[test]
    fn import_row_from_excel() {
        let excel_row = AttendanceExcelRow {
            source_row: 12,
            student_identifier: "3A-李雷".into(),
            status_text: "病假".into(),
            minutes_value: Some("45".into()),
            note: Some("感冒".into()),
        };

        let row = AttendanceImportRow::try_from(excel_row).unwrap();
        assert_eq!(row.status, AttendanceStatus::Excused);
        assert_eq!(row.minutes_attended, Some(45));
        assert_eq!(row.note.as_deref(), Some("感冒"));
    }

    #[test]
    fn batch_validation_requires_rows() {
        let class_id = Uuid::new_v4();
        let meeting_date = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let session = AttendanceSessionKey::new(class_id, meeting_date, 1).unwrap();

        let meeting_id = Uuid::new_v4();

        assert!(matches!(
            AttendanceImportBatch::new(session, meeting_id, None, Vec::new(), None),
            Err(AttendanceValidationError::EmptyBatch)
        ));

        let row = AttendanceImportRow {
            source_row: 3,
            student_identifier: "student-a".into(),
            enrollment_id: None,
            status: AttendanceStatus::Present,
            minutes_attended: None,
            note: None,
        };

        assert!(
            AttendanceImportBatch::new(session, meeting_id, Some("Bob".into()), vec![row], None)
                .is_ok()
        );
    }
}
