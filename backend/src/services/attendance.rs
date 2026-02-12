use std::collections::{HashMap, HashSet};

use chrono::{NaiveDate, Utc};
use uuid::Uuid;

use crate::domain::{
    AttendanceExcelRow, AttendanceImportBatch, AttendanceImportRow, AttendanceRecord,
    AttendanceSessionKey, AttendanceStatus, ClassInstance, StudentProfile,
};
use crate::error::AppError;
use crate::utils::excel::{ExcelWorkbook, Worksheet};

/// 默认模板列顺序：班级、课次、日期、学生标识、姓名、状态、出勤分钟、备注。
const TEMPLATE_HEADERS: [&str; 8] = [
    "Class",
    "Session",
    "Date",
    "Student Identifier",
    "Student Name",
    "Status",
    "Minutes",
    "Note",
];

#[derive(Debug, Default, Clone)]
pub struct AttendanceService;

impl AttendanceService {
    pub fn new() -> Self {
        Self::default()
    }

    /// 生成考勤空模板，后续由 utils/excel 写入实际文件。
    pub fn generate_template(
        &self,
        class: &ClassInstance,
        session_dates: &[NaiveDate],
        roster: &[StudentProfile],
    ) -> AttendanceTemplate {
        let mut rows = Vec::new();
        for (session_index, meeting_date) in session_dates.iter().enumerate() {
            let session_number = (session_index + 1).to_string();
            for student in roster {
                rows.push(vec![
                    class.class_code.clone(),
                    session_number.clone(),
                    meeting_date.to_string(),
                    format!("{}-{}", student.original_class, student.name),
                    student.name.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                ]);
            }
        }

        AttendanceTemplate {
            worksheet: Worksheet {
                name: format!("{}-{}", class.class_code, class.weekday),
                rows: build_template_rows(rows),
            },
        }
    }

    /// 从 Excel 工作簿解析考勤行，并依据 roster lookup 补齐 enrollment_id。
    pub fn parse_workbook(
        &self,
        workbook: ExcelWorkbook,
        session: AttendanceSessionKey,
        class_meeting_id: Uuid,
        options: AttendanceImportOptions,
    ) -> Result<AttendanceImportBatch, AppError> {
        let sheet = workbook.primary_sheet();
        let filter = ImportFilterSet::new(options.placeholders, options.ignored_identifiers);
        let mut parsed_rows = Vec::new();

        for (idx, row) in sheet.rows.iter().enumerate().skip(1) {
            let identifier = row.get(3).cloned().unwrap_or_default();
            if filter.should_skip(&identifier) {
                continue;
            }

            let excel_row = AttendanceExcelRow {
                source_row: (idx + 1) as u32,
                student_identifier: identifier,
                status_text: row.get(5).cloned().unwrap_or_default(),
                minutes_value: row.get(6).cloned(),
                note: row.get(7).cloned(),
            };

            match AttendanceImportRow::try_from(excel_row) {
                Ok(import_row) => {
                    let key = import_row.identifier_key();
                    if let Some(enrollment_id) = options.roster_lookup.get(&key) {
                        parsed_rows.push(import_row.with_enrollment(*enrollment_id));
                    } else {
                        return Err(AppError::Validation(format!(
                            "Excel 第 {} 行无法匹配到报名记录: {}",
                            import_row.source_row, import_row.student_identifier
                        )));
                    }
                }
                Err(err) => {
                    return Err(AppError::Validation(err.to_string()));
                }
            }
        }

        AttendanceImportBatch::new(
            session,
            class_meeting_id,
            options.recorded_by,
            parsed_rows,
            options.import_job_id,
        )
        .map_err(|err| AppError::Validation(err.to_string()))
    }

    /// 根据历史记录生成插入与更新计划，确保幂等。
    pub fn plan_persistence(
        &self,
        batch: &AttendanceImportBatch,
        history: &AttendanceHistory,
    ) -> AttendancePersistPlan {
        let mut inserts = Vec::new();
        let mut updates = Vec::new();
        let mut skipped = Vec::new();
        let recorded_by = batch.recorded_by.clone();
        let recorded_at = batch.submitted_at;

        for row in &batch.rows {
            if let Some(enrollment_id) = row.enrollment_id {
                let key = (batch.class_meeting_id, enrollment_id);
                if let Some(existing) = history.records.get(&key) {
                    if existing.status != row.status
                        || existing.minutes_attended != row.minutes_attended
                    {
                        let mut updated = existing.clone();
                        updated.status = row.status;
                        updated.minutes_attended = row.minutes_attended;
                        updated.recorded_by = recorded_by.clone();
                        updated.recorded_at = recorded_at;
                        updates.push(updated);
                    }
                } else {
                    inserts.push(AttendanceRecord {
                        id: Uuid::new_v4(),
                        class_meeting_id: batch.class_meeting_id,
                        enrollment_id,
                        status: row.status,
                        minutes_attended: row.minutes_attended,
                        recorded_by: recorded_by.clone(),
                        recorded_at,
                    });
                }
            } else {
                skipped.push(row.clone());
            }
        }

        AttendancePersistPlan {
            inserts,
            updates,
            skipped,
        }
    }

    pub fn build_roster_lookup(entries: &[AttendanceRosterEntry]) -> HashMap<String, Uuid> {
        entries
            .iter()
            .map(|entry| {
                (
                    normalize_key(&entry.student_identifier),
                    entry.enrollment_id,
                )
            })
            .collect()
    }

    /// 兼容现有 API：直接接受考勤记录并假装写库。
    pub async fn record_bulk(&self, _records: Vec<AttendanceRecord>) -> Result<(), AppError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AttendanceTemplate {
    pub worksheet: Worksheet,
}

impl AttendanceTemplate {
    pub fn headers(&self) -> &[String] {
        &self.worksheet.rows[0]
    }

    pub fn rows(&self) -> &[Vec<String>] {
        &self.worksheet.rows
    }
}

#[derive(Debug, Clone)]
pub struct AttendanceRosterEntry {
    pub enrollment_id: Uuid,
    pub student_identifier: String,
}

#[derive(Debug, Clone)]
pub struct AttendanceImportOptions<'a> {
    pub recorded_by: Option<String>,
    pub import_job_id: Option<Uuid>,
    pub placeholders: &'a [String],
    pub ignored_identifiers: &'a [String],
    pub roster_lookup: &'a HashMap<String, Uuid>,
}

impl<'a> AttendanceImportOptions<'a> {
    pub fn new(
        recorded_by: Option<String>,
        import_job_id: Option<Uuid>,
        placeholders: &'a [String],
        ignored_identifiers: &'a [String],
        roster_lookup: &'a HashMap<String, Uuid>,
    ) -> Self {
        Self {
            recorded_by,
            import_job_id,
            placeholders,
            ignored_identifiers,
            roster_lookup,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AttendanceHistory {
    records: HashMap<(Uuid, Uuid), AttendanceRecord>,
}

impl AttendanceHistory {
    pub fn new(records: Vec<AttendanceRecord>) -> Self {
        Self {
            records: records
                .into_iter()
                .map(|record| ((record.class_meeting_id, record.enrollment_id), record))
                .collect(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AttendancePersistPlan {
    pub inserts: Vec<AttendanceRecord>,
    pub updates: Vec<AttendanceRecord>,
    pub skipped: Vec<AttendanceImportRow>,
}

fn build_template_rows(mut rows: Vec<Vec<String>>) -> Vec<Vec<String>> {
    let mut result = Vec::with_capacity(rows.len() + 1);
    result.push(TEMPLATE_HEADERS.iter().map(|v| v.to_string()).collect());
    result.append(&mut rows);
    result
}

struct ImportFilterSet {
    placeholders: HashSet<String>,
    ignored: HashSet<String>,
}

impl ImportFilterSet {
    fn new(placeholders: &[String], ignored_identifiers: &[String]) -> Self {
        Self {
            placeholders: normalize_set(placeholders),
            ignored: normalize_set(ignored_identifiers),
        }
    }

    fn should_skip(&self, identifier: &str) -> bool {
        let key = normalize_key(identifier);
        key.is_empty() || self.placeholders.contains(&key) || self.ignored.contains(&key)
    }
}

fn normalize_set(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .map(|value| normalize_key(value))
        .filter(|value| !value.is_empty())
        .collect()
}

fn normalize_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime};

    use super::*;

    fn fake_class() -> ClassInstance {
        ClassInstance {
            id: Uuid::new_v4(),
            term_id: Uuid::new_v4(),
            campus_id: Uuid::new_v4(),
            club_id: Uuid::new_v4(),
            class_code: "A1".into(),
            weekday: 1,
            start_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            location: None,
            capacity: None,
            status: crate::domain::ClassStatus::Planned,
            notes: None,
        }
    }

    fn fake_roster() -> Vec<StudentProfile> {
        vec![
            StudentProfile {
                id: Uuid::new_v4(),
                name: "李雷".into(),
                original_class: "3A".into(),
                is_teacher_child: false,
            },
            StudentProfile {
                id: Uuid::new_v4(),
                name: "韩梅梅".into(),
                original_class: "3A".into(),
                is_teacher_child: false,
            },
        ]
    }

    #[test]
    fn template_contains_header_and_rows() {
        let service = AttendanceService::new();
        let class = fake_class();
        let roster = fake_roster();
        let dates = vec![
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(),
        ];

        let template = service.generate_template(&class, &dates, &roster);
        assert_eq!(template.rows().len(), roster.len() * dates.len() + 1);
        assert_eq!(template.headers().len(), TEMPLATE_HEADERS.len());
        assert_eq!(template.rows()[1][3], "3A-李雷");
    }

    #[test]
    fn parse_workbook_assigns_enrollment_and_filters_placeholder() {
        let service = AttendanceService::new();
        let roster_entries = vec![AttendanceRosterEntry {
            enrollment_id: Uuid::new_v4(),
            student_identifier: "3A-李雷".into(),
        }];
        let roster_lookup = AttendanceService::build_roster_lookup(&roster_entries);
        let sheet = Worksheet {
            name: "Sheet1".into(),
            rows: vec![
                TEMPLATE_HEADERS.iter().map(|v| v.to_string()).collect(),
                vec![
                    "A1".into(),
                    "1".into(),
                    "2026-03-01".into(),
                    "3A-李雷".into(),
                    "李雷".into(),
                    "请假".into(),
                    "45".into(),
                    "note".into(),
                ],
                vec![
                    "A1".into(),
                    "1".into(),
                    "2026-03-01".into(),
                    "(跳过)".into(),
                    "".into(),
                    "P".into(),
                    "".into(),
                    "".into(),
                ],
            ],
        };
        let workbook = ExcelWorkbook {
            sheets: vec![sheet],
        };
        let session = AttendanceSessionKey::new(
            fake_class().id,
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            1,
        )
        .unwrap();
        let class_meeting_id = Uuid::new_v4();
        let placeholder = String::from("(跳过)");
        let options = AttendanceImportOptions::new(
            Some("Alice".into()),
            None,
            std::slice::from_ref(&placeholder),
            &[],
            &roster_lookup,
        );

        let batch = service
            .parse_workbook(workbook, session, class_meeting_id, options)
            .expect("parse success");
        assert_eq!(batch.rows.len(), 1);
        assert!(batch.rows[0].enrollment_id.is_some());
        assert_eq!(batch.rows[0].status, AttendanceStatus::Leave);
    }

    #[test]
    fn plan_persistence_detects_updates_and_inserts() {
        let service = AttendanceService::new();
        let class_meeting_id = Uuid::new_v4();
        let existing_record = AttendanceRecord {
            id: Uuid::new_v4(),
            class_meeting_id,
            enrollment_id: Uuid::new_v4(),
            status: AttendanceStatus::Present,
            minutes_attended: Some(90),
            recorded_by: Some("Alice".into()),
            recorded_at: Utc::now(),
        };
        let history = AttendanceHistory::new(vec![existing_record.clone()]);

        let session = AttendanceSessionKey::new(
            Uuid::new_v4(),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            1,
        )
        .unwrap();
        let rows = vec![
            AttendanceImportRow {
                source_row: 2,
                student_identifier: "3A-李雷".into(),
                enrollment_id: Some(existing_record.enrollment_id),
                status: AttendanceStatus::Absent,
                minutes_attended: Some(0),
                note: None,
            },
            AttendanceImportRow {
                source_row: 3,
                student_identifier: "3A-韩梅梅".into(),
                enrollment_id: Some(Uuid::new_v4()),
                status: AttendanceStatus::Present,
                minutes_attended: Some(90),
                note: None,
            },
        ];
        let batch =
            AttendanceImportBatch::new(session, class_meeting_id, Some("Bob".into()), rows, None)
                .unwrap();

        let plan = service.plan_persistence(&batch, &history);
        assert_eq!(plan.updates.len(), 1);
        assert_eq!(plan.inserts.len(), 1);
        assert!(plan.skipped.is_empty());
    }
}
