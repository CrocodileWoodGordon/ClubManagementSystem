use std::collections::{HashMap, HashSet};

use chrono::{NaiveDate, NaiveTime};
use uuid::Uuid;

use crate::domain::{
    AttendanceExcelRow, AttendanceImportBatch, AttendanceImportRow, AttendanceRecord,
    AttendanceSessionKey, AttendanceStatus, ClassInstance, StudentProfile,
};
use crate::error::AppError;
use crate::utils::excel::{CellMerge, ExcelWorkbook, Worksheet};

const TEMPLATE_TOTAL_COLUMNS: usize = 20;
const TEMPLATE_WEEK_COLUMNS: usize = 18;
const TEMPLATE_UID_COLUMN_INDEX: usize = 1;
const TEMPLATE_FIRST_WEEK_COLUMN_INDEX: usize = 2;
const TEMPLATE_HEADER_ROW_INDEX: usize = 3;
const TEMPLATE_INSTRUCTION_TEXT: &str = "将考勤情况为缺席的同学对应单元格删除（设为空），考勤情况正常的同学无需更改。如果课时过多，直接将后面多余课时所有同学的对应单元格删除（设置为空）即可。机器读取，请不要擅自改动文件其余部分。如有成员增减，请重新获取最新的考勤表。";
const TEMPLATE_DEFAULT_STATUS: &str = "正常";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttendanceWeekWindow {
    start_week: u16,
    end_week: u16,
}

impl AttendanceWeekWindow {
    pub fn new(start_week: u16, end_week: u16) -> Result<Self, AppError> {
        if start_week == 0 || end_week == 0 {
            return Err(AppError::Validation("周次必须从 1 开始".into()));
        }
        if start_week > end_week {
            return Err(AppError::Validation("起始周不能大于结束周".into()));
        }
        if end_week > TEMPLATE_WEEK_COLUMNS as u16 {
            return Err(AppError::Validation(format!(
                "结束周超出模板最大范围 (1-{})",
                TEMPLATE_WEEK_COLUMNS
            )));
        }
        Ok(Self {
            start_week,
            end_week,
        })
    }

    pub fn from_optional(start_week: Option<u16>, end_week: Option<u16>) -> Result<Self, AppError> {
        let default_start = 1;
        let default_end = TEMPLATE_WEEK_COLUMNS as u16;
        let start = start_week.unwrap_or(default_start);
        let end = end_week.unwrap_or(default_end);
        Self::new(start, end)
    }

    pub fn includes(&self, week_number: u16) -> bool {
        week_number >= self.start_week && week_number <= self.end_week
    }
}

impl Default for AttendanceWeekWindow {
    fn default() -> Self {
        Self {
            start_week: 1,
            end_week: TEMPLATE_WEEK_COLUMNS as u16,
        }
    }
}

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
        _session_dates: &[NaiveDate],
        roster: &[StudentProfile],
        context: AttendanceTemplateContext,
        week_window: AttendanceWeekWindow,
    ) -> AttendanceTemplate {
        let sheet_name = format!("{}-{}", class.class_code, class.weekday);
        let club_name = context.club_or_default();
        let campus_name = context.campus_or_default();

        let mut rows = Vec::with_capacity(roster.len() + 4);
        rows.push(build_title_row(&class.class_code));
        rows.push(build_meta_row(&club_name, &campus_name, class));
        rows.push(build_instruction_row());
        rows.push(build_header_row(&week_window));

        for (index, student) in roster.iter().enumerate() {
            rows.push(build_student_row(index, student, &week_window));
        }

        AttendanceTemplate {
            worksheet: Worksheet {
                name: sheet_name,
                rows,
                merged_cells: build_default_merges(),
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
        let status_col_index = resolve_status_column(session.session_number)?;

        for (idx, row) in sheet.rows.iter().enumerate().skip(4) {
            let identifier = row
                .get(TEMPLATE_UID_COLUMN_INDEX)
                .cloned()
                .unwrap_or_default();
            if filter.should_skip(&identifier) {
                continue;
            }

            let status_text = row
                .get(status_col_index)
                .cloned()
                .unwrap_or_else(|| TEMPLATE_DEFAULT_STATUS.to_string());

            let excel_row = AttendanceExcelRow {
                source_row: (idx + 1) as u32,
                student_identifier: identifier,
                status_text,
                minutes_value: None,
                note: None,
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

        let consolidated_rows = consolidate_duplicate_rows(&batch.rows);

        for row in consolidated_rows {
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
                skipped.push(row);
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
    #[allow(dead_code)]
    pub async fn record_bulk(&self, _records: Vec<AttendanceRecord>) -> Result<(), AppError> {
        Ok(())
    }
}

fn consolidate_duplicate_rows(rows: &[AttendanceImportRow]) -> Vec<AttendanceImportRow> {
    let mut result = Vec::new();
    let mut merged: HashMap<Uuid, AttendanceImportRow> = HashMap::new();
    let mut order: Vec<Uuid> = Vec::new();

    for row in rows {
        if let Some(enrollment_id) = row.enrollment_id {
            if let Some(existing) = merged.get_mut(&enrollment_id) {
                let existing_rank = status_severity(existing.status);
                let new_rank = status_severity(row.status);
                if new_rank > existing_rank {
                    *existing = row.clone();
                } else if new_rank == existing_rank {
                    if existing.minutes_attended.is_none() && row.minutes_attended.is_some() {
                        existing.minutes_attended = row.minutes_attended;
                    }
                    if existing.note.is_none() && row.note.is_some() {
                        existing.note = row.note.clone();
                    }
                }
            } else {
                order.push(enrollment_id);
                merged.insert(enrollment_id, row.clone());
            }
        } else {
            result.push(row.clone());
        }
    }

    for enrollment_id in order {
        if let Some(row) = merged.remove(&enrollment_id) {
            result.push(row);
        }
    }

    result
}

fn status_severity(status: AttendanceStatus) -> u8 {
    // 缺勤 > 请假 > 病假 > 正常，用于挑选最严重状态。
    match status {
        AttendanceStatus::Present => 0,
        AttendanceStatus::Excused => 1,
        AttendanceStatus::Leave => 2,
        AttendanceStatus::Absent => 3,
    }
}

#[derive(Debug, Clone)]
pub struct AttendanceTemplateContext {
    pub club_name: String,
    pub campus_name: String,
}

impl AttendanceTemplateContext {
    pub fn new(club_name: impl Into<String>, campus_name: impl Into<String>) -> Self {
        Self {
            club_name: club_name.into().trim().to_string(),
            campus_name: campus_name.into().trim().to_string(),
        }
    }

    fn club_or_default(&self) -> String {
        if self.club_name.is_empty() {
            "未命名社团".into()
        } else {
            self.club_name.clone()
        }
    }

    fn campus_or_default(&self) -> String {
        if self.campus_name.is_empty() {
            "未命名校区".into()
        } else {
            self.campus_name.clone()
        }
    }
}

#[derive(Debug, Clone)]
pub struct AttendanceTemplate {
    pub worksheet: Worksheet,
}

impl AttendanceTemplate {
    #[allow(dead_code)]
    pub fn headers(&self) -> &[String] {
        &self.worksheet.rows[TEMPLATE_HEADER_ROW_INDEX]
    }

    #[allow(dead_code)]
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

fn build_title_row(class_name: &str) -> Vec<String> {
    let mut row = blank_row();
    row[0] = class_name.to_string();
    row
}

fn build_meta_row(club_name: &str, campus_name: &str, class: &ClassInstance) -> Vec<String> {
    let mut row = blank_row();
    row[0] = format!("社团：{}", club_name);
    row[8] = format!("校区：{}", campus_name);
    row[12] = format!("上课时间：{}", format_class_schedule(class));
    row
}

fn build_instruction_row() -> Vec<String> {
    let mut row = blank_row();
    row[0] = TEMPLATE_INSTRUCTION_TEXT.to_string();
    row
}

fn build_header_row(week_window: &AttendanceWeekWindow) -> Vec<String> {
    let mut row = Vec::with_capacity(TEMPLATE_TOTAL_COLUMNS);
    row.push("编号".into());
    row.push("学生UID".into());
    for week in 1..=TEMPLATE_WEEK_COLUMNS {
        if week_window.includes(week as u16) {
            row.push(format!("第{}周", week));
        } else {
            row.push(String::new());
        }
    }
    row
}

fn build_student_row(
    index: usize,
    student: &StudentProfile,
    week_window: &AttendanceWeekWindow,
) -> Vec<String> {
    let mut row = Vec::with_capacity(TEMPLATE_TOTAL_COLUMNS);
    row.push((index + 1).to_string());
    row.push(format!("{}-{}", student.original_class, student.name));
    for week in 1..=TEMPLATE_WEEK_COLUMNS {
        if week_window.includes(week as u16) {
            row.push(TEMPLATE_DEFAULT_STATUS.to_string());
        } else {
            row.push(String::new());
        }
    }
    row
}

fn build_default_merges() -> Vec<CellMerge> {
    vec![
        CellMerge::new(1, 1, 1, TEMPLATE_TOTAL_COLUMNS),
        CellMerge::new(2, 1, 2, 8),
        CellMerge::new(2, 9, 2, 12),
        CellMerge::new(2, 13, 2, TEMPLATE_TOTAL_COLUMNS),
        CellMerge::new(3, 1, 3, TEMPLATE_TOTAL_COLUMNS),
    ]
}

fn blank_row() -> Vec<String> {
    vec![String::new(); TEMPLATE_TOTAL_COLUMNS]
}

fn format_class_schedule(class: &ClassInstance) -> String {
    let weekday = weekday_label(class.weekday);
    let start = format_time(class.start_time);
    let end = format_time(class.end_time);
    format!("{} {}-{}", weekday, start, end)
}

fn format_time(value: NaiveTime) -> String {
    value.format("%H:%M").to_string()
}

fn weekday_label(weekday: u8) -> &'static str {
    match weekday {
        1 => "周一",
        2 => "周二",
        3 => "周三",
        4 => "周四",
        5 => "周五",
        6 => "周六",
        7 => "周日",
        _ => "周?",
    }
}

fn resolve_status_column(session_number: u16) -> Result<usize, AppError> {
    if session_number == 0 {
        return Err(AppError::Validation("课次编号无效".into()));
    }
    let session_index = session_number as usize;
    if session_index > TEMPLATE_WEEK_COLUMNS {
        return Err(AppError::Validation(format!(
            "课次编号 {} 超出可导出模板范围 (1-{})",
            session_number, TEMPLATE_WEEK_COLUMNS
        )));
    }
    Ok(TEMPLATE_FIRST_WEEK_COLUMN_INDEX + session_index - 1)
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
    use chrono::{NaiveDate, NaiveTime, Utc};

    use super::*;
    use crate::domain::AttendanceStatus;

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
        let context = AttendanceTemplateContext::new("机器人社", "主校区");

        let week_window = AttendanceWeekWindow::default();
        let template = service.generate_template(&class, &dates, &roster, context, week_window);
        assert_eq!(template.rows().len(), roster.len() + 4);
        assert_eq!(template.headers().len(), TEMPLATE_TOTAL_COLUMNS);
        let first_student_row = TEMPLATE_HEADER_ROW_INDEX + 1;
        assert_eq!(
            template.rows()[first_student_row][TEMPLATE_UID_COLUMN_INDEX],
            "3A-李雷"
        );
        assert_eq!(template.worksheet.merged_cells.len(), 5);
        assert_eq!(template.headers()[2], "第1周");
    }

    #[test]
    fn template_respects_custom_week_window() {
        let service = AttendanceService::new();
        let class = fake_class();
        let roster = fake_roster();
        let dates = vec![NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()];
        let context = AttendanceTemplateContext::new("机器人社", "主校区");

        let week_window = AttendanceWeekWindow::new(3, 6).expect("week range valid");
        let template = service.generate_template(&class, &dates, &roster, context, week_window);

        let headers = template.headers();
        assert_eq!(headers[TEMPLATE_FIRST_WEEK_COLUMN_INDEX], "");
        assert_eq!(headers[TEMPLATE_FIRST_WEEK_COLUMN_INDEX + 1], "");
        assert_eq!(headers[TEMPLATE_FIRST_WEEK_COLUMN_INDEX + 2], "第3周");
        assert_eq!(headers[TEMPLATE_FIRST_WEEK_COLUMN_INDEX + 5], "第6周");
        assert_eq!(headers[TEMPLATE_FIRST_WEEK_COLUMN_INDEX + 6], "");

        let first_student_row = TEMPLATE_HEADER_ROW_INDEX + 1;
        assert_eq!(
            template.rows()[first_student_row][TEMPLATE_FIRST_WEEK_COLUMN_INDEX],
            ""
        );
        assert_eq!(
            template.rows()[first_student_row][TEMPLATE_FIRST_WEEK_COLUMN_INDEX + 2],
            TEMPLATE_DEFAULT_STATUS
        );
        assert_eq!(
            template.rows()[first_student_row][TEMPLATE_FIRST_WEEK_COLUMN_INDEX + 6],
            ""
        );
    }

    #[test]
    fn parse_workbook_assigns_enrollment_and_filters_placeholder() {
        let service = AttendanceService::new();
        let roster_entries = vec![AttendanceRosterEntry {
            enrollment_id: Uuid::new_v4(),
            student_identifier: "3A-李雷".into(),
        }];
        let roster_lookup = AttendanceService::build_roster_lookup(&roster_entries);

        let class = fake_class();
        let roster = fake_roster();
        let dates = vec![NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()];
        let context = AttendanceTemplateContext::new("机器人社", "主校区");
        let week_window = AttendanceWeekWindow::default();
        let mut worksheet = service
            .generate_template(&class, &dates, &roster, context, week_window)
            .worksheet;

        let first_student_row = TEMPLATE_HEADER_ROW_INDEX + 1;
        worksheet.rows[first_student_row][TEMPLATE_FIRST_WEEK_COLUMN_INDEX] = "请假".into();
        let placeholder_row = TEMPLATE_HEADER_ROW_INDEX + 2;
        worksheet.rows[placeholder_row][TEMPLATE_UID_COLUMN_INDEX] = "(跳过)".into();

        let workbook = ExcelWorkbook {
            sheets: vec![worksheet],
        };
        let session =
            AttendanceSessionKey::new(class.id, NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(), 1)
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

    #[test]
    fn plan_persistence_merges_duplicates_with_severe_status() {
        let service = AttendanceService::new();
        let class_meeting_id = Uuid::new_v4();
        let session = AttendanceSessionKey::new(
            Uuid::new_v4(),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            1,
        )
        .unwrap();
        let enrollment_id = Uuid::new_v4();

        let rows = vec![
            AttendanceImportRow {
                source_row: 5,
                student_identifier: "3A-李雷".into(),
                enrollment_id: Some(enrollment_id),
                status: AttendanceStatus::Present,
                minutes_attended: Some(90),
                note: None,
            },
            AttendanceImportRow {
                source_row: 6,
                student_identifier: "3A-李雷".into(),
                enrollment_id: Some(enrollment_id),
                status: AttendanceStatus::Leave,
                minutes_attended: None,
                note: Some("家庭原因".into()),
            },
            AttendanceImportRow {
                source_row: 7,
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

        let plan = service.plan_persistence(&batch, &AttendanceHistory::default());
        assert_eq!(plan.inserts.len(), 2);
        let merged = plan
            .inserts
            .iter()
            .find(|record| record.enrollment_id == enrollment_id)
            .expect("merged record exists");
        assert_eq!(merged.status, AttendanceStatus::Leave);
        assert_eq!(merged.recorded_by.as_deref(), Some("Bob"));
    }

    #[test]
    fn plan_persistence_keeps_present_when_duplicates_all_present() {
        let service = AttendanceService::new();
        let class_meeting_id = Uuid::new_v4();
        let session = AttendanceSessionKey::new(
            Uuid::new_v4(),
            NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(),
            2,
        )
        .unwrap();
        let enrollment_id = Uuid::new_v4();

        let rows = vec![
            AttendanceImportRow {
                source_row: 8,
                student_identifier: "3A-李雷".into(),
                enrollment_id: Some(enrollment_id),
                status: AttendanceStatus::Present,
                minutes_attended: None,
                note: None,
            },
            AttendanceImportRow {
                source_row: 9,
                student_identifier: "3A-李雷".into(),
                enrollment_id: Some(enrollment_id),
                status: AttendanceStatus::Present,
                minutes_attended: Some(90),
                note: Some("补登".into()),
            },
        ];
        let batch =
            AttendanceImportBatch::new(session, class_meeting_id, Some("Carol".into()), rows, None)
                .unwrap();

        let plan = service.plan_persistence(&batch, &AttendanceHistory::default());
        assert_eq!(plan.inserts.len(), 1);
        let record = &plan.inserts[0];
        assert_eq!(record.status, AttendanceStatus::Present);
        assert_eq!(record.minutes_attended, Some(90));
        assert_eq!(record.recorded_by.as_deref(), Some("Carol"));
    }
}
