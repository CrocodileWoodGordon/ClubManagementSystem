use axum::{
    Json, Router,
    extract::{Multipart, Path, Query, State},
    routing::{get, post},
};
use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use uuid::Uuid;

use crate::{
    api::ApiState,
    domain::{
        AttendanceRecord, AttendanceSessionKey, AttendanceStatus, ClassInstance, ClassStatus,
        StudentProfile,
    },
    error::AppError,
    services::attendance::{
        AttendanceHistory, AttendanceImportOptions, AttendancePersistPlan, AttendanceRosterEntry,
        AttendanceService,
    },
    utils::excel::ExcelWorkbook,
};

const DEFAULT_PLACEHOLDERS: &[&str] = &["(跳过)"];

#[derive(Debug, Deserialize)]
pub struct AttendanceQuery {
    pub class_id: Uuid,
    pub class_meeting_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct AttendanceListResponse {
    pub data: Vec<AttendanceRecordDto>,
}

#[derive(Debug, Serialize)]
pub struct AttendanceRecordDto {
    pub id: Uuid,
    pub class_meeting_id: Uuid,
    pub meeting_date: NaiveDate,
    pub session_number: u16,
    pub enrollment_id: Uuid,
    pub student_id: Uuid,
    pub student_name: String,
    pub student_identifier: String,
    pub status: AttendanceStatus,
    pub minutes_attended: Option<i32>,
    pub recorded_by: Option<String>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct AttendanceTemplateResponse {
    pub class: ClassOverview,
    pub meetings: Vec<ClassMeetingDto>,
    pub worksheet: WorksheetDto,
}

#[derive(Debug, Serialize)]
pub struct ClassOverview {
    pub id: Uuid,
    pub term_id: Uuid,
    pub campus_id: Uuid,
    pub club_id: Uuid,
    pub class_code: String,
    pub weekday: u8,
    pub start_time: String,
    pub end_time: String,
    pub location: Option<String>,
    pub capacity: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ClassMeetingDto {
    pub id: Uuid,
    pub meeting_date: NaiveDate,
    pub session_number: u16,
}

#[derive(Debug, Serialize)]
pub struct WorksheetDto {
    pub name: String,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct AttendanceImportResponse {
    pub batch_id: Uuid,
    pub inserted: usize,
    pub updated: usize,
    pub skipped: Vec<AttendanceImportRowDto>,
}

#[derive(Debug, Serialize)]
pub struct AttendanceImportRowDto {
    pub source_row: u32,
    pub student_identifier: String,
    pub status: AttendanceStatus,
    pub minutes_attended: Option<i32>,
    pub note: Option<String>,
}

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/", get(list_attendance))
        .route("/template/{class_id}", get(download_template))
        .route("/import", post(import_attendance))
}

async fn list_attendance(
    State(state): State<ApiState>,
    Query(query): Query<AttendanceQuery>,
) -> Result<Json<AttendanceListResponse>, AppError> {
    let data = fetch_attendance_rows(&state, query.class_id, query.class_meeting_id).await?;
    Ok(Json(AttendanceListResponse { data }))
}

async fn download_template(
    State(state): State<ApiState>,
    Path(class_id): Path<Uuid>,
) -> Result<Json<AttendanceTemplateResponse>, AppError> {
    let class = fetch_class(&state, class_id).await?;
    let meetings = ensure_class_meetings(&state, &class).await?;
    if meetings.is_empty() {
        return Err(AppError::Validation(
            "该班级尚未创建上课安排，无法生成模板".into(),
        ));
    }

    let roster_rows = fetch_roster_rows(&state, class_id).await?;
    if roster_rows.is_empty() {
        return Err(AppError::Validation(
            "班级尚未分配学生，无法生成模板".into(),
        ));
    }

    let session_dates: Vec<NaiveDate> = meetings.iter().map(|m| m.meeting_date).collect();
    let roster_profiles: Vec<StudentProfile> = roster_rows
        .iter()
        .map(|row| StudentProfile {
            id: row.student_id,
            name: row.student_name.clone(),
            original_class: row.homeroom_name.clone(),
            is_teacher_child: row.is_teacher_child,
        })
        .collect();

    let service = AttendanceService::new();
    let template = service.generate_template(&class, &session_dates, &roster_profiles);

    let meetings_dto = meetings
        .iter()
        .map(|meeting| {
            let session_number = u16::try_from(meeting.session_number)
                .map_err(|_| AppError::Validation("课次编号异常".into()))?;
            Ok(ClassMeetingDto {
                id: meeting.id,
                meeting_date: meeting.meeting_date,
                session_number,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    let response = AttendanceTemplateResponse {
        class: ClassOverview::from(&class),
        meetings: meetings_dto,
        worksheet: WorksheetDto {
            name: template.worksheet.name,
            rows: template.worksheet.rows,
        },
    };
    Ok(Json(response))
}

async fn import_attendance(
    State(state): State<ApiState>,
    mut multipart: Multipart,
) -> Result<Json<AttendanceImportResponse>, AppError> {
    let upload = read_upload_payload(&mut multipart).await?;
    let context = fetch_meeting_context(&state, upload.class_meeting_id).await?;
    if context.meeting.class_id != context.class.id {
        return Err(AppError::Validation("课次与班级不一致".into()));
    }
    let roster_rows = fetch_roster_rows(&state, context.class.id).await?;
    if roster_rows.is_empty() {
        return Err(AppError::Validation("班级没有学生，无法导入考勤".into()));
    }

    let roster_entries: Vec<AttendanceRosterEntry> = roster_rows
        .iter()
        .map(|row| AttendanceRosterEntry {
            enrollment_id: row.enrollment_id,
            student_identifier: format_identifier(&row.homeroom_name, &row.student_name),
        })
        .collect();
    let roster_lookup = AttendanceService::build_roster_lookup(&roster_entries);
    let placeholders: Vec<String> = DEFAULT_PLACEHOLDERS.iter().map(|v| v.to_string()).collect();
    let ignored = upload.ignored_identifiers;

    let session_number: u16 = context
        .meeting
        .session_number
        .try_into()
        .map_err(|_| AppError::Validation("课次编号异常".into()))?;
    let session_key = AttendanceSessionKey::new(
        context.class.id,
        context.meeting.meeting_date,
        session_number,
    )
    .map_err(|err| AppError::Validation(err.to_string()))?;

    let workbook = ExcelWorkbook::from_bytes(upload.bytes, Some(&upload.filename))?;
    let service = AttendanceService::new();
    let options = AttendanceImportOptions::new(
        upload.recorded_by.clone(),
        None,
        &placeholders,
        &ignored,
        &roster_lookup,
    );
    let batch = service
        .parse_workbook(workbook, session_key, context.meeting.id, options)
        .map_err(|err| AppError::Validation(err.to_string()))?;

    let history = load_attendance_history(&state, context.meeting.id).await?;
    let plan = service.plan_persistence(&batch, &history);
    persist_plan(&state, &plan).await?;

    let response = AttendanceImportResponse {
        batch_id: batch.batch_id,
        inserted: plan.inserts.len(),
        updated: plan.updates.len(),
        skipped: plan
            .skipped
            .iter()
            .map(|row| AttendanceImportRowDto {
                source_row: row.source_row,
                student_identifier: row.student_identifier.clone(),
                status: row.status,
                minutes_attended: row.minutes_attended,
                note: row.note.clone(),
            })
            .collect(),
    };
    Ok(Json(response))
}

async fn fetch_attendance_rows(
    state: &ApiState,
    class_id: Uuid,
    meeting_id: Option<Uuid>,
) -> Result<Vec<AttendanceRecordDto>, AppError> {
    let mut query = String::from(
        r#"
        SELECT ar.id,
               ar.class_meeting_id,
               ar.enrollment_id,
               ar.status,
               ar.minutes_attended,
               ar.recorded_by,
               ar.recorded_at,
               cm.meeting_date,
               cm.session_number,
               e.student_id,
               s.full_name AS student_name,
               h.display_name AS homeroom_name
        FROM attendance_records ar
        INNER JOIN class_meetings cm ON cm.id = ar.class_meeting_id
        INNER JOIN enrollments e ON e.id = ar.enrollment_id
        INNER JOIN students s ON s.id = e.student_id
        INNER JOIN homerooms h ON h.id = s.homeroom_id
        WHERE e.class_id = $1
    "#,
    );

    if meeting_id.is_some() {
        query.push_str(" AND ar.class_meeting_id = $2");
    }
    query.push_str(" ORDER BY cm.meeting_date, s.full_name");

    let rows = if let Some(meeting_id) = meeting_id {
        sqlx::query(&query)
            .bind(class_id)
            .bind(meeting_id)
            .fetch_all(&state.pool)
            .await
    } else {
        sqlx::query(&query)
            .bind(class_id)
            .fetch_all(&state.pool)
            .await
    }
    .map_err(|err| AppError::Database(err.to_string()))?;

    rows.into_iter()
        .map(|row| {
            let status_text: String = row.try_get("status").map_err(db_err)?;
            let status =
                AttendanceStatus::try_from(status_text.as_str()).map_err(|err| validation(err))?;
            let session_number_raw: i16 = row.try_get("session_number").map_err(db_err)?;
            let session_number = u16::try_from(session_number_raw)
                .map_err(|_| AppError::Validation("课次编号异常".into()))?;
            let student_name: String = row.try_get("student_name").map_err(db_err)?;
            let homeroom_name: String = row.try_get("homeroom_name").map_err(db_err)?;
            Ok(AttendanceRecordDto {
                id: row.try_get("id").map_err(db_err)?,
                class_meeting_id: row.try_get("class_meeting_id").map_err(db_err)?,
                meeting_date: row.try_get("meeting_date").map_err(db_err)?,
                session_number,
                enrollment_id: row.try_get("enrollment_id").map_err(db_err)?,
                student_id: row.try_get("student_id").map_err(db_err)?,
                student_name: student_name.clone(),
                student_identifier: format_identifier(&homeroom_name, &student_name),
                status,
                minutes_attended: row.try_get("minutes_attended").map_err(db_err)?,
                recorded_by: row.try_get("recorded_by").map_err(db_err)?,
                recorded_at: row.try_get("recorded_at").map_err(db_err)?,
            })
        })
        .collect()
}

async fn fetch_class(state: &ApiState, class_id: Uuid) -> Result<ClassInstance, AppError> {
    let row = sqlx::query_as::<_, ClassRow>(
        r#"
            SELECT id,
                   term_id,
                   campus_id,
                   club_id,
                   class_code,
                   weekday,
                   start_time,
                   end_time,
                   location,
                   capacity,
                   status,
                   notes
            FROM classes
            WHERE id = $1
        "#,
    )
    .bind(class_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    if let Some(row) = row {
        Ok(row.into_instance())
    } else {
        Err(AppError::NotFound("班级不存在或已被删除".into()))
    }
}

async fn fetch_class_meetings(
    state: &ApiState,
    class_id: Uuid,
) -> Result<Vec<ClassMeetingRow>, AppError> {
    let rows = sqlx::query_as::<_, ClassMeetingRow>(
        r#"
            SELECT id, class_id, meeting_date, session_number
            FROM class_meetings
            WHERE class_id = $1
            ORDER BY meeting_date ASC
        "#,
    )
    .bind(class_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;
    Ok(rows)
}

async fn ensure_class_meetings(
    state: &ApiState,
    class: &ClassInstance,
) -> Result<Vec<ClassMeetingRow>, AppError> {
    let mut meetings = fetch_class_meetings(state, class.id).await?;
    if !meetings.is_empty() {
        return Ok(meetings);
    }

    let term = fetch_term_window(state, class.term_id).await?;
    let dates = build_meeting_dates(class.weekday, &term)?;
    if dates.is_empty() {
        return Err(AppError::Validation(
            "该班级所在学期范围内没有匹配的上课日期，无法生成模板".into(),
        ));
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    for (index, meeting_date) in dates.iter().enumerate() {
        let session_number = i16::try_from(index + 1)
            .map_err(|_| AppError::Validation("课次数量超出支持范围".into()))?;
        let meeting_id = Uuid::new_v4();
        sqlx::query(
            r#"
                INSERT INTO class_meetings (id, class_id, meeting_date, session_number)
                VALUES ($1,$2,$3,$4)
                ON CONFLICT (class_id, meeting_date) DO NOTHING
            "#,
        )
        .bind(meeting_id)
        .bind(class.id)
        .bind(meeting_date)
        .bind(session_number)
        .execute(&mut *tx)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    meetings = fetch_class_meetings(state, class.id).await?;
    Ok(meetings)
}

async fn fetch_term_window(state: &ApiState, term_id: Uuid) -> Result<TermWindow, AppError> {
    let row = sqlx::query_as::<_, TermWindow>(
        r#"
            SELECT start_date, end_date
            FROM terms
            WHERE id = $1
        "#,
    )
    .bind(term_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    if let Some(window) = row {
        if window.start_date > window.end_date {
            return Err(AppError::Validation("学期的起止日期不合法".into()));
        }
        Ok(window)
    } else {
        Err(AppError::NotFound("未找到对应的学期信息".into()))
    }
}

fn build_meeting_dates(weekday: u8, term: &TermWindow) -> Result<Vec<NaiveDate>, AppError> {
    if !(1..=7).contains(&weekday) {
        return Err(AppError::Validation("班级的上课星期不合法".into()));
    }
    if term.start_date > term.end_date {
        return Err(AppError::Validation("学期的起止日期不合法".into()));
    }

    let target = u32::from(weekday);
    let mut current = term.start_date;
    let start_weekday = current.weekday().number_from_monday();
    let offset = (target + 7 - start_weekday) % 7;
    current += Duration::days(offset as i64);

    if current > term.end_date {
        return Ok(Vec::new());
    }

    let mut dates = Vec::new();
    let mut day = current;
    while day <= term.end_date {
        dates.push(day);
        day += Duration::days(7);
    }
    Ok(dates)
}

async fn fetch_roster_rows(state: &ApiState, class_id: Uuid) -> Result<Vec<RosterRow>, AppError> {
    let rows = sqlx::query_as::<_, RosterRow>(
        r#"
            SELECT e.id AS enrollment_id,
                   e.student_id,
                   s.full_name AS student_name,
                   h.display_name AS homeroom_name,
                   s.is_teacher_child
            FROM enrollments e
            INNER JOIN students s ON s.id = e.student_id
            INNER JOIN homerooms h ON h.id = s.homeroom_id
            WHERE e.class_id = $1
              AND e.status IN ('PENDING','ACTIVE')
            ORDER BY h.display_name, s.full_name
        "#,
    )
    .bind(class_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;
    Ok(rows)
}

async fn fetch_meeting_context(
    state: &ApiState,
    class_meeting_id: Uuid,
) -> Result<MeetingContext, AppError> {
    let row = sqlx::query_as::<_, MeetingJoinRow>(
        r#"
            SELECT cm.id AS meeting_id,
                   cm.class_id,
                   cm.meeting_date,
                   cm.session_number,
                   c.term_id,
                   c.campus_id,
                   c.club_id,
                   c.class_code,
                   c.weekday,
                   c.start_time,
                   c.end_time,
                   c.location,
                   c.capacity,
                   c.status,
                   c.notes
            FROM class_meetings cm
            INNER JOIN classes c ON c.id = cm.class_id
            WHERE cm.id = $1
        "#,
    )
    .bind(class_meeting_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    if let Some(row) = row {
        let class = ClassInstance {
            id: row.class_id,
            term_id: row.term_id,
            campus_id: row.campus_id,
            club_id: row.club_id,
            class_code: row.class_code.clone(),
            weekday: row.weekday as u8,
            start_time: row.start_time,
            end_time: row.end_time,
            location: row.location.clone(),
            capacity: row.capacity,
            status: ClassStatus::from_str(&row.status),
            notes: row.notes.clone(),
        };
        Ok(MeetingContext {
            meeting: ClassMeetingRow {
                id: row.meeting_id,
                class_id: row.class_id,
                meeting_date: row.meeting_date,
                session_number: row.session_number,
            },
            class,
        })
    } else {
        Err(AppError::NotFound("未找到对应的课次记录".into()))
    }
}

async fn load_attendance_history(
    state: &ApiState,
    class_meeting_id: Uuid,
) -> Result<AttendanceHistory, AppError> {
    let rows = sqlx::query_as::<_, crate::db::models::AttendanceRecordRow>(
        r#"
            SELECT id,
                   class_meeting_id,
                   enrollment_id,
                   status,
                   minutes_attended,
                   recorded_by,
                   recorded_at
            FROM attendance_records
            WHERE class_meeting_id = $1
        "#,
    )
    .bind(class_meeting_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    let records = rows
        .into_iter()
        .map(AttendanceRecord::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| AppError::Validation(err.to_string()))?;
    Ok(AttendanceHistory::new(records))
}

async fn persist_plan(state: &ApiState, plan: &AttendancePersistPlan) -> Result<(), AppError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    for record in &plan.inserts {
        sqlx::query(
            r#"
                INSERT INTO attendance_records (
                    id, class_meeting_id, enrollment_id, status,
                    minutes_attended, recorded_by, recorded_at
                ) VALUES ($1,$2,$3,$4,$5,$6,$7)
            "#,
        )
        .bind(record.id)
        .bind(record.class_meeting_id)
        .bind(record.enrollment_id)
        .bind(record.status.as_str())
        .bind(record.minutes_attended)
        .bind(record.recorded_by.clone())
        .bind(record.recorded_at)
        .execute(&mut *tx)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;
    }

    for record in &plan.updates {
        sqlx::query(
            r#"
                UPDATE attendance_records
                SET status = $1,
                    minutes_attended = $2,
                    recorded_by = $3,
                    recorded_at = $4
                WHERE id = $5
            "#,
        )
        .bind(record.status.as_str())
        .bind(record.minutes_attended)
        .bind(record.recorded_by.clone())
        .bind(record.recorded_at)
        .bind(record.id)
        .execute(&mut *tx)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|err| AppError::Database(err.to_string()))
}

async fn read_upload_payload(
    multipart: &mut Multipart,
) -> Result<AttendanceUploadPayload, AppError> {
    let mut bytes: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut class_meeting_id: Option<Uuid> = None;
    let mut recorded_by: Option<String> = None;
    let mut ignored_identifiers: Vec<String> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::Validation(format!("读取上传字段失败: {}", err)))?
    {
        match field.name() {
            Some("class_meeting_id") => {
                let value = field.text().await.map_err(|err| {
                    AppError::Validation(format!("读取 class_meeting_id 失败: {}", err))
                })?;
                let id = Uuid::parse_str(value.trim())
                    .map_err(|_| AppError::Validation("class_meeting_id 需为 UUID 格式".into()))?;
                class_meeting_id = Some(id);
            }
            Some("recorded_by") => {
                let value = field.text().await.map_err(|err| {
                    AppError::Validation(format!("读取 recorded_by 失败: {}", err))
                })?;
                if !value.trim().is_empty() {
                    recorded_by = Some(value.trim().to_string());
                }
            }
            Some("ignored_identifiers") => {
                let value = field.text().await.map_err(|err| {
                    AppError::Validation(format!("读取 ignored_identifiers 失败: {}", err))
                })?;
                ignored_identifiers.extend(parse_identifier_list(&value));
            }
            _ => {
                if field.file_name().is_some() || field.name() == Some("file") {
                    let name = field
                        .file_name()
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| "attendance.xlsx".into());
                    let file_bytes = field.bytes().await.map_err(|err| {
                        AppError::Validation(format!("读取 Excel 内容失败: {}", err))
                    })?;
                    bytes = Some(file_bytes.to_vec());
                    filename = Some(name);
                }
            }
        }
    }

    let file_bytes =
        bytes.ok_or_else(|| AppError::Validation("请上传 Excel 文件（字段名 file）".into()))?;
    let class_meeting_id = class_meeting_id
        .ok_or_else(|| AppError::Validation("缺少 class_meeting_id 字段".into()))?;

    Ok(AttendanceUploadPayload {
        bytes: file_bytes,
        filename: filename.unwrap_or_else(|| "attendance.xlsx".into()),
        class_meeting_id,
        recorded_by,
        ignored_identifiers,
    })
}

fn parse_identifier_list(value: &str) -> Vec<String> {
    if value.trim().is_empty() {
        return Vec::new();
    }

    if let Ok(list) = serde_json::from_str::<Vec<String>>(value) {
        return list
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect();
    }

    value
        .split(|c| c == ',' || c == '\n' || c == ';')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .collect()
}

fn format_identifier(homeroom: &str, student: &str) -> String {
    format!("{}-{}", homeroom, student)
}

fn db_err(err: sqlx::Error) -> AppError {
    AppError::Database(err.to_string())
}

fn validation(err: impl ToString) -> AppError {
    AppError::Validation(err.to_string())
}

#[derive(Debug, FromRow)]
struct ClassRow {
    id: Uuid,
    term_id: Uuid,
    campus_id: Uuid,
    club_id: Uuid,
    class_code: String,
    weekday: i16,
    start_time: NaiveTime,
    end_time: NaiveTime,
    location: Option<String>,
    capacity: Option<i32>,
    status: String,
    notes: Option<String>,
}

impl ClassRow {
    fn into_instance(self) -> ClassInstance {
        ClassInstance {
            id: self.id,
            term_id: self.term_id,
            campus_id: self.campus_id,
            club_id: self.club_id,
            class_code: self.class_code,
            weekday: self.weekday as u8,
            start_time: self.start_time,
            end_time: self.end_time,
            location: self.location,
            capacity: self.capacity,
            status: ClassStatus::from_str(&self.status),
            notes: self.notes,
        }
    }
}

#[derive(Debug, FromRow)]
struct ClassMeetingRow {
    id: Uuid,
    class_id: Uuid,
    meeting_date: NaiveDate,
    session_number: i16,
}

#[derive(Debug, FromRow)]
struct RosterRow {
    enrollment_id: Uuid,
    student_id: Uuid,
    student_name: String,
    homeroom_name: String,
    is_teacher_child: bool,
}

#[derive(Debug, FromRow)]
struct MeetingJoinRow {
    meeting_id: Uuid,
    class_id: Uuid,
    meeting_date: NaiveDate,
    session_number: i16,
    term_id: Uuid,
    campus_id: Uuid,
    club_id: Uuid,
    class_code: String,
    weekday: i16,
    start_time: NaiveTime,
    end_time: NaiveTime,
    location: Option<String>,
    capacity: Option<i32>,
    status: String,
    notes: Option<String>,
}

struct MeetingContext {
    meeting: ClassMeetingRow,
    class: ClassInstance,
}

#[derive(Debug, FromRow)]
struct TermWindow {
    start_date: NaiveDate,
    end_date: NaiveDate,
}

impl From<&ClassInstance> for ClassOverview {
    fn from(class: &ClassInstance) -> Self {
        ClassOverview {
            id: class.id,
            term_id: class.term_id,
            campus_id: class.campus_id,
            club_id: class.club_id,
            class_code: class.class_code.clone(),
            weekday: class.weekday,
            start_time: format_time(class.start_time),
            end_time: format_time(class.end_time),
            location: class.location.clone(),
            capacity: class.capacity,
            notes: class.notes.clone(),
        }
    }
}

fn format_time(value: NaiveTime) -> String {
    format!("{:02}:{:02}", value.hour(), value.minute())
}

#[derive(Debug)]
struct AttendanceUploadPayload {
    bytes: Vec<u8>,
    filename: String,
    class_meeting_id: Uuid,
    recorded_by: Option<String>,
    ignored_identifiers: Vec<String>,
}
