use std::collections::{HashMap, HashSet};

use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Row, postgres::PgRow};
use uuid::Uuid;

use crate::{db::DbPool, error::AppError, utils::excel::ExcelWorkbook};

#[derive(Debug, Clone, Copy)]
struct TermContext {
    id: Uuid,
    academic_year: i16,
}

#[derive(Debug)]
pub struct StudentRosterService<'a> {
    pool: &'a DbPool,
}

#[derive(Debug, Default)]
pub struct HomeroomListFilters {
    pub term_id: Option<Uuid>,
    pub campus_id: Option<Uuid>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HomeroomRosterDto {
    pub id: Uuid,
    pub term_id: Uuid,
    pub campus_id: Uuid,
    pub campus_name: String,
    pub academic_year: i16,
    pub display_name: String,
    pub grade_label: String,
    pub class_label: String,
    pub head_teacher_name: Option<String>,
    pub head_teacher_phone: Option<String>,
    pub notes: Option<String>,
    pub student_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StudentRecordDto {
    pub id: Uuid,
    pub homeroom_id: Uuid,
    pub full_name: String,
    pub student_code: Option<String>,
    pub is_teacher_child: bool,
    pub primary_guardian_name: Option<String>,
    pub primary_guardian_phone: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CloneRosterResult {
    pub copied_homerooms: i64,
    pub copied_students: i64,
}

#[derive(Debug, Serialize)]
pub struct TeacherChildImportSummary {
    pub total_rows: i32,
    pub matched_students: i32,
    pub updated_students: i32,
    pub already_marked: i32,
    pub skipped_rows: i32,
    pub duplicate_rows: i32,
    pub errors: Vec<TeacherChildImportError>,
}

#[derive(Debug, Serialize)]
pub struct TeacherChildImportError {
    pub row: u32,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct HomeroomUpdateChanges {
    pub display_name: Option<String>,
    pub grade_label: Option<String>,
    pub class_label: Option<String>,
    pub head_teacher_name: Option<String>,
    pub head_teacher_phone: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug)]
pub struct NewStudentInput {
    pub full_name: String,
    pub student_code: Option<String>,
    pub is_teacher_child: bool,
    pub primary_guardian_name: Option<String>,
    pub primary_guardian_phone: Option<String>,
}

#[derive(Debug, Default)]
pub struct UpdateStudentChanges {
    pub homeroom_id: Option<Uuid>,
    pub full_name: Option<String>,
    pub student_code: Option<String>,
    pub is_teacher_child: Option<bool>,
    pub primary_guardian_name: Option<String>,
    pub primary_guardian_phone: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug)]
pub struct CloneRosterRequest {
    pub source_term_id: Uuid,
    pub target_term_id: Uuid,
    pub campus_id: Option<Uuid>,
}

impl<'a> StudentRosterService<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn list_homerooms(
        &self,
        filters: &HomeroomListFilters,
    ) -> Result<Vec<HomeroomRosterDto>, AppError> {
        let term = self.resolve_term(filters.term_id).await?;
        let mut builder = QueryBuilder::new(
            r#"
            SELECT h.id,
                   h.term_id,
                   h.campus_id,
                   cam.name AS campus_name,
                   h.academic_year,
                   h.display_name,
                   h.grade_label,
                   h.class_label,
                   h.head_teacher_name,
                   h.head_teacher_phone,
                   h.notes,
                   COALESCE(stats.student_count, 0) AS student_count
            FROM homerooms h
            INNER JOIN campuses cam ON cam.id = h.campus_id
            LEFT JOIN LATERAL (
                SELECT COUNT(*)::bigint AS student_count
                FROM students s
                WHERE s.homeroom_id = h.id
                  AND s.status = 'ACTIVE'
            ) stats ON true
            WHERE h.term_id = "#,
        );
        builder.push_bind(term.id);

        if let Some(campus_id) = filters.campus_id {
            builder.push(" AND h.campus_id = ").push_bind(campus_id);
        }
        if let Some(search) = filters.search.as_ref().and_then(|value| non_empty(value)) {
            let like = format!("%{}%", search);
            builder
                .push(" AND (h.display_name ILIKE ")
                .push_bind(like.clone())
                .push(" OR h.grade_label ILIKE ")
                .push_bind(like.clone())
                .push(" OR h.class_label ILIKE ")
                .push_bind(like)
                .push(")");
        }

        builder.push(" ORDER BY h.grade_label, h.class_label, h.display_name");

        let rows = builder
            .build()
            .fetch_all(self.pool)
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        rows.into_iter()
            .map(map_homeroom_row)
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn get_homeroom(
        &self,
        homeroom_id: Uuid,
        term_id: Option<Uuid>,
    ) -> Result<HomeroomRosterDto, AppError> {
        let term = self.resolve_term(term_id).await?;
        self.fetch_homeroom(homeroom_id, term.id).await
    }

    pub async fn update_homeroom(
        &self,
        homeroom_id: Uuid,
        term_id: Option<Uuid>,
        changes: HomeroomUpdateChanges,
    ) -> Result<HomeroomRosterDto, AppError> {
        if !changes.has_updates() {
            return Err(AppError::Validation("请至少提供一个需要更新的字段".into()));
        }
        let term = self.resolve_term(term_id).await?;
        let row = sqlx::query(
            r#"
                UPDATE homerooms
                SET display_name = COALESCE($3, display_name),
                    grade_label = COALESCE($4, grade_label),
                    class_label = COALESCE($5, class_label),
                    head_teacher_name = COALESCE($6, head_teacher_name),
                    head_teacher_phone = COALESCE($7, head_teacher_phone),
                    notes = COALESCE($8, notes)
                WHERE id = $1 AND term_id = $2
                RETURNING id, term_id, campus_id, academic_year, display_name, grade_label,
                          class_label, head_teacher_name, head_teacher_phone, notes,
                          (SELECT name FROM campuses WHERE id = homerooms.campus_id) AS campus_name,
                          (SELECT COUNT(*)::bigint FROM students WHERE homeroom_id = homerooms.id AND status = 'ACTIVE') AS student_count
            "#,
        )
        .bind(homeroom_id)
        .bind(term.id)
        .bind(changes.display_name)
        .bind(changes.grade_label)
        .bind(changes.class_label)
        .bind(changes.head_teacher_name)
        .bind(changes.head_teacher_phone)
        .bind(changes.notes)
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let Some(row) = row else {
            return Err(AppError::NotFound("未找到对应班级或不属于该学期".into()));
        };

        map_homeroom_row(row)
    }

    pub async fn list_students(
        &self,
        homeroom_id: Uuid,
        term_id: Option<Uuid>,
    ) -> Result<Vec<StudentRecordDto>, AppError> {
        let term = self.resolve_term(term_id).await?;
        self.ensure_homeroom_in_term(homeroom_id, term.id).await?;

        let rows = sqlx::query(
            r#"
                SELECT id, homeroom_id, full_name, student_code, is_teacher_child,
                       primary_guardian_name, primary_guardian_phone, status
                FROM students
                WHERE homeroom_id = $1 AND status = 'ACTIVE'
                ORDER BY full_name ASC
            "#,
        )
        .bind(homeroom_id)
        .fetch_all(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        rows.into_iter().map(map_student_row).collect()
    }

    pub async fn create_student(
        &self,
        homeroom_id: Uuid,
        term_id: Option<Uuid>,
        input: NewStudentInput,
    ) -> Result<StudentRecordDto, AppError> {
        if input.full_name.trim().is_empty() {
            return Err(AppError::Validation("学生姓名不能为空".into()));
        }
        let term = self.resolve_term(term_id).await?;
        self.ensure_homeroom_in_term(homeroom_id, term.id).await?;

        let row = sqlx::query(
            r#"
                INSERT INTO students (full_name, student_code, homeroom_id, is_teacher_child,
                                      primary_guardian_name, primary_guardian_phone)
                VALUES ($1,$2,$3,$4,$5,$6)
                RETURNING id, homeroom_id, full_name, student_code, is_teacher_child,
                          primary_guardian_name, primary_guardian_phone, status
            "#,
        )
        .bind(input.full_name.trim())
        .bind(input.student_code.as_deref())
        .bind(homeroom_id)
        .bind(input.is_teacher_child)
        .bind(input.primary_guardian_name.as_deref())
        .bind(input.primary_guardian_phone.as_deref())
        .fetch_one(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        map_student_row(row)
    }

    pub async fn update_student(
        &self,
        student_id: Uuid,
        term_id: Option<Uuid>,
        changes: UpdateStudentChanges,
    ) -> Result<StudentRecordDto, AppError> {
        if !changes.has_updates() {
            return Err(AppError::Validation("请至少提供一个需要更新的字段".into()));
        }
        let term = self.resolve_term(term_id).await?;
        let current = sqlx::query(
            r#"
                SELECT s.homeroom_id
                FROM students s
                INNER JOIN homerooms h ON h.id = s.homeroom_id
                WHERE s.id = $1 AND h.term_id = $2
            "#,
        )
        .bind(student_id)
        .bind(term.id)
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let Some(row) = current else {
            return Err(AppError::NotFound("未找到对应学生或不属于该学期".into()));
        };

        let current_homeroom: Uuid = row
            .try_get("homeroom_id")
            .map_err(|err| AppError::Database(err.to_string()))?;

        let target_homeroom = if let Some(new_id) = changes.homeroom_id {
            self.ensure_homeroom_in_term(new_id, term.id).await?;
            new_id
        } else {
            current_homeroom
        };

        let row = sqlx::query(
            r#"
                UPDATE students
                SET homeroom_id = $2,
                    full_name = COALESCE($3, full_name),
                    student_code = COALESCE($4, student_code),
                    is_teacher_child = COALESCE($5, is_teacher_child),
                    primary_guardian_name = COALESCE($6, primary_guardian_name),
                    primary_guardian_phone = COALESCE($7, primary_guardian_phone),
                    status = COALESCE($8, status)
                WHERE id = $1
                RETURNING id, homeroom_id, full_name, student_code, is_teacher_child,
                          primary_guardian_name, primary_guardian_phone, status
            "#,
        )
        .bind(student_id)
        .bind(target_homeroom)
        .bind(changes.full_name.as_deref())
        .bind(changes.student_code.as_deref())
        .bind(changes.is_teacher_child)
        .bind(changes.primary_guardian_name.as_deref())
        .bind(changes.primary_guardian_phone.as_deref())
        .bind(changes.status.as_deref())
        .fetch_one(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        map_student_row(row)
    }

    pub async fn delete_student(
        &self,
        student_id: Uuid,
        term_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        let term = self.resolve_term(term_id).await?;
        let updated = sqlx::query(
            r#"
                UPDATE students
                SET status = 'INACTIVE'
                WHERE id = $1
                  AND homeroom_id IN (SELECT id FROM homerooms WHERE term_id = $2)
            "#,
        )
        .bind(student_id)
        .bind(term.id)
        .execute(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        if updated.rows_affected() == 0 {
            return Err(AppError::NotFound("未找到对应学生或不属于该学期".into()));
        }
        Ok(())
    }

    pub async fn clone_roster(
        &self,
        request: CloneRosterRequest,
    ) -> Result<CloneRosterResult, AppError> {
        if request.source_term_id == request.target_term_id {
            return Err(AppError::Validation("源学期与目标学期不能相同".into()));
        }
        let source = self.resolve_term(Some(request.source_term_id)).await?;
        let target = self.resolve_term(Some(request.target_term_id)).await?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        let existing: i64 = sqlx::query_scalar(
            r#"
                SELECT COUNT(*)::bigint
                FROM homerooms
                WHERE term_id = $1
                  AND ($2::uuid IS NULL OR campus_id = $2)
            "#,
        )
        .bind(target.id)
        .bind(request.campus_id)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        if existing > 0 {
            return Err(AppError::Validation(
                "目标学期在该校区已存在学生班级数据，请先清理后再复用".into(),
            ));
        }

        let homeroom_rows = sqlx::query(
            r#"
                SELECT id, campus_id, grade_label, class_label, display_name,
                       head_teacher_name, head_teacher_phone, notes
                FROM homerooms
                WHERE term_id = $1
                  AND ($2::uuid IS NULL OR campus_id = $2)
            "#,
        )
        .bind(source.id)
        .bind(request.campus_id)
        .fetch_all(tx.as_mut())
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        if homeroom_rows.is_empty() {
            return Err(AppError::Validation("源学期暂无可复制的班级数据".into()));
        }

        let mut mapping = HashMap::new();
        for row in homeroom_rows {
            let id: Uuid = row
                .try_get("id")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let campus_id: Uuid = row
                .try_get("campus_id")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let grade_label: String = row
                .try_get("grade_label")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let class_label: String = row
                .try_get("class_label")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let display_name: String = row
                .try_get("display_name")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let head_teacher_name: Option<String> = row
                .try_get("head_teacher_name")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let head_teacher_phone: Option<String> = row
                .try_get("head_teacher_phone")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let notes: Option<String> = row
                .try_get("notes")
                .map_err(|err| AppError::Database(err.to_string()))?;

            let inserted = sqlx::query(
                r#"
                    INSERT INTO homerooms (term_id, campus_id, academic_year, grade_label, class_label,
                                            display_name, head_teacher_name, head_teacher_phone, notes)
                    VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                    RETURNING id
                "#,
            )
            .bind(target.id)
            .bind(campus_id)
            .bind(target.academic_year)
            .bind(&grade_label)
            .bind(&class_label)
            .bind(&display_name)
            .bind(head_teacher_name.as_deref())
            .bind(head_teacher_phone.as_deref())
            .bind(notes.as_deref())
            .fetch_one(tx.as_mut())
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

            let new_id: Uuid = inserted
                .try_get("id")
                .map_err(|err| AppError::Database(err.to_string()))?;
            mapping.insert(id, new_id);
        }

        let source_ids: Vec<Uuid> = mapping.keys().copied().collect();
        let mut copied_students = 0i64;
        if !source_ids.is_empty() {
            let student_rows = sqlx::query(
                r#"
                    SELECT homeroom_id, full_name, student_code, is_teacher_child,
                           primary_guardian_name, primary_guardian_phone
                    FROM students
                    WHERE homeroom_id = ANY($1)
                      AND status = 'ACTIVE'
                "#,
            )
            .bind(&source_ids)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

            for row in student_rows {
                let homeroom_id: Uuid = row
                    .try_get("homeroom_id")
                    .map_err(|err| AppError::Database(err.to_string()))?;
                let Some(target_homeroom) = mapping.get(&homeroom_id) else {
                    continue;
                };
                let full_name: String = row
                    .try_get("full_name")
                    .map_err(|err| AppError::Database(err.to_string()))?;
                let student_code: Option<String> = row
                    .try_get("student_code")
                    .map_err(|err| AppError::Database(err.to_string()))?;
                let is_teacher_child: bool = row
                    .try_get("is_teacher_child")
                    .map_err(|err| AppError::Database(err.to_string()))?;
                let guardian_name: Option<String> = row
                    .try_get("primary_guardian_name")
                    .map_err(|err| AppError::Database(err.to_string()))?;
                let guardian_phone: Option<String> = row
                    .try_get("primary_guardian_phone")
                    .map_err(|err| AppError::Database(err.to_string()))?;

                sqlx::query(
                    r#"
                        INSERT INTO students (full_name, student_code, homeroom_id, is_teacher_child,
                                              primary_guardian_name, primary_guardian_phone)
                        VALUES ($1,$2,$3,$4,$5,$6)
                    "#,
                )
                .bind(full_name)
                .bind(student_code.as_deref())
                .bind(*target_homeroom)
                .bind(is_teacher_child)
                .bind(guardian_name.as_deref())
                .bind(guardian_phone.as_deref())
                .execute(tx.as_mut())
                .await
                .map_err(|err| AppError::Database(err.to_string()))?;

                copied_students += 1;
            }
        }

        tx.commit()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        Ok(CloneRosterResult {
            copied_homerooms: mapping.len() as i64,
            copied_students,
        })
    }

    pub async fn import_teacher_children(
        &self,
        term_id: Option<Uuid>,
        campus_id: Uuid,
        workbook: ExcelWorkbook,
        raw_config: Option<&str>,
    ) -> Result<TeacherChildImportSummary, AppError> {
        let term = self.resolve_term(term_id).await?;
        self.ensure_campus_exists(campus_id).await?;
        let columns = TeacherChildImportColumns::from_json(raw_config)?;
        let parsed = parse_teacher_child_drafts(&workbook, &columns);
        if parsed.drafts.is_empty() {
            return Err(AppError::Validation(
                "Excel 文件为空，未发现班级/学生数据".into(),
            ));
        }

        let homeroom_rows = sqlx::query(
            r#"
                SELECT id, display_name
                FROM homerooms
                WHERE term_id = $1 AND campus_id = $2
            "#,
        )
        .bind(term.id)
        .bind(campus_id)
        .fetch_all(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let mut homeroom_index = HashMap::new();
        for row in homeroom_rows {
            let id: Uuid = row
                .try_get("id")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let display_name: String = row
                .try_get("display_name")
                .map_err(|err| AppError::Database(err.to_string()))?;
            homeroom_index.insert(normalize_label(&display_name), id);
        }

        let student_rows = sqlx::query(
            r#"
                SELECT s.id, s.homeroom_id, s.full_name, s.student_code, s.is_teacher_child
                FROM students s
                INNER JOIN homerooms h ON h.id = s.homeroom_id
                WHERE h.term_id = $1 AND h.campus_id = $2 AND s.status = 'ACTIVE'
            "#,
        )
        .bind(term.id)
        .bind(campus_id)
        .fetch_all(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let mut students_by_name: HashMap<String, IndexedStudent> = HashMap::new();
        let mut students_by_code: HashMap<String, IndexedStudent> = HashMap::new();
        for row in student_rows {
            let record = IndexedStudent {
                id: row
                    .try_get("id")
                    .map_err(|err| AppError::Database(err.to_string()))?,
                homeroom_id: row
                    .try_get("homeroom_id")
                    .map_err(|err| AppError::Database(err.to_string()))?,
                is_teacher_child: row
                    .try_get("is_teacher_child")
                    .map_err(|err| AppError::Database(err.to_string()))?,
            };
            let full_name: String = row
                .try_get("full_name")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let name_key = student_name_key(record.homeroom_id, &full_name);
            students_by_name.insert(name_key, record.clone());

            let student_code: Option<String> = row
                .try_get("student_code")
                .map_err(|err| AppError::Database(err.to_string()))?;
            if let Some(code) = student_code {
                if !code.trim().is_empty() {
                    let code_key = student_code_key(record.homeroom_id, &code);
                    students_by_code.insert(code_key, record.clone());
                }
            }
        }

        let mut seen_students = HashSet::new();
        let mut to_update = Vec::new();
        let mut summary =
            TeacherChildImportSummary::new(parsed.drafts.len() as i32, parsed.skipped_rows);

        for draft in parsed.drafts {
            if draft.class_name.trim().is_empty() {
                summary.record_error(draft.row_number, "班级列为空，无法匹配学生");
                continue;
            }
            if draft.student_name.trim().is_empty() {
                summary.record_error(draft.row_number, "姓名列为空，无法匹配学生");
                continue;
            }

            let Some(&homeroom_id) = homeroom_index.get(&normalize_label(&draft.class_name)) else {
                summary.record_error(
                    draft.row_number,
                    format!(
                        "未找到班级 `{}`，请确认名称与学生名册一致",
                        draft.class_name
                    ),
                );
                continue;
            };

            let student = draft
                .student_code
                .as_ref()
                .and_then(|code| students_by_code.get(&student_code_key(homeroom_id, code)))
                .or_else(|| {
                    students_by_name.get(&student_name_key(homeroom_id, &draft.student_name))
                });

            let Some(student) = student else {
                summary.record_error(
                    draft.row_number,
                    format!(
                        "未找到学生 `{}`（班级：{}），请确认姓名/班级是否一致",
                        draft.student_name, draft.class_name
                    ),
                );
                continue;
            };

            summary.matched_students += 1;
            if !seen_students.insert(student.id) {
                summary.duplicate_rows += 1;
                continue;
            }

            if student.is_teacher_child {
                summary.already_marked += 1;
            } else {
                summary.updated_students += 1;
                to_update.push(student.id);
            }
        }

        if !to_update.is_empty() {
            let mut builder =
                QueryBuilder::new("UPDATE students SET is_teacher_child = true WHERE id IN (");
            let mut separated = builder.separated(", ");
            for id in &to_update {
                separated.push_bind(id);
            }
            builder.push(") AND is_teacher_child = false");
            builder
                .build()
                .execute(self.pool)
                .await
                .map_err(|err| AppError::Database(err.to_string()))?;
        }

        Ok(summary)
    }

    async fn fetch_homeroom(
        &self,
        homeroom_id: Uuid,
        term_id: Uuid,
    ) -> Result<HomeroomRosterDto, AppError> {
        let row = sqlx::query(
            r#"
                SELECT h.id,
                       h.term_id,
                       h.campus_id,
                       cam.name AS campus_name,
                       h.academic_year,
                       h.display_name,
                       h.grade_label,
                       h.class_label,
                       h.head_teacher_name,
                       h.head_teacher_phone,
                       h.notes,
                       COALESCE(stats.student_count, 0) AS student_count
                FROM homerooms h
                INNER JOIN campuses cam ON cam.id = h.campus_id
                LEFT JOIN LATERAL (
                    SELECT COUNT(*)::bigint AS student_count
                    FROM students s
                    WHERE s.homeroom_id = h.id AND s.status = 'ACTIVE'
                ) stats ON true
                WHERE h.id = $1 AND h.term_id = $2
            "#,
        )
        .bind(homeroom_id)
        .bind(term_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let Some(row) = row else {
            return Err(AppError::NotFound("未找到对应班级或不属于该学期".into()));
        };

        map_homeroom_row(row)
    }

    async fn ensure_homeroom_in_term(
        &self,
        homeroom_id: Uuid,
        term_id: Uuid,
    ) -> Result<(), AppError> {
        let exists: Option<i64> = sqlx::query_scalar(
            r#"
                SELECT 1::bigint
                FROM homerooms
                WHERE id = $1 AND term_id = $2
            "#,
        )
        .bind(homeroom_id)
        .bind(term_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        if exists.is_none() {
            return Err(AppError::Validation("班级不属于该学期".into()));
        }
        Ok(())
    }

    async fn ensure_campus_exists(&self, campus_id: Uuid) -> Result<(), AppError> {
        let exists: Option<i64> = sqlx::query_scalar(
            r#"
                SELECT 1::bigint
                FROM campuses
                WHERE id = $1
            "#,
        )
        .bind(campus_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        if exists.is_none() {
            return Err(AppError::Validation("指定的校区不存在".into()));
        }
        Ok(())
    }

    async fn resolve_term(&self, provided: Option<Uuid>) -> Result<TermContext, AppError> {
        if let Some(term_id) = provided {
            return self.fetch_term_by_id(term_id).await;
        }

        let row = sqlx::query(
            r#"
                SELECT id, start_date
                FROM terms
                WHERE is_active = true
                ORDER BY enrollment_start DESC
                LIMIT 1
            "#,
        )
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let Some(row) = row else {
            return Err(AppError::Validation(
                "未找到激活学期，请先创建并激活学期".into(),
            ));
        };

        let id: Uuid = row
            .try_get("id")
            .map_err(|err| AppError::Database(err.to_string()))?;
        let start_date: chrono::NaiveDate = row
            .try_get("start_date")
            .map_err(|err| AppError::Database(err.to_string()))?;
        let academic_year = start_date.year() as i16;
        Ok(TermContext { id, academic_year })
    }

    async fn fetch_term_by_id(&self, term_id: Uuid) -> Result<TermContext, AppError> {
        let row = sqlx::query(
            r#"
                SELECT id, start_date
                FROM terms
                WHERE id = $1
            "#,
        )
        .bind(term_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let Some(row) = row else {
            return Err(AppError::Validation("指定的学期不存在".into()));
        };

        let start_date: chrono::NaiveDate = row
            .try_get("start_date")
            .map_err(|err| AppError::Database(err.to_string()))?;
        let academic_year = start_date.year() as i16;
        Ok(TermContext {
            id: term_id,
            academic_year,
        })
    }
}

fn map_homeroom_row(row: PgRow) -> Result<HomeroomRosterDto, AppError> {
    Ok(HomeroomRosterDto {
        id: row
            .try_get("id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        term_id: row
            .try_get("term_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        campus_id: row
            .try_get("campus_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        campus_name: row
            .try_get("campus_name")
            .map_err(|err| AppError::Database(err.to_string()))?,
        academic_year: row
            .try_get("academic_year")
            .map_err(|err| AppError::Database(err.to_string()))?,
        display_name: row
            .try_get("display_name")
            .map_err(|err| AppError::Database(err.to_string()))?,
        grade_label: row
            .try_get("grade_label")
            .map_err(|err| AppError::Database(err.to_string()))?,
        class_label: row
            .try_get("class_label")
            .map_err(|err| AppError::Database(err.to_string()))?,
        head_teacher_name: row
            .try_get("head_teacher_name")
            .map_err(|err| AppError::Database(err.to_string()))?,
        head_teacher_phone: row
            .try_get("head_teacher_phone")
            .map_err(|err| AppError::Database(err.to_string()))?,
        notes: row
            .try_get("notes")
            .map_err(|err| AppError::Database(err.to_string()))?,
        student_count: row
            .try_get("student_count")
            .map_err(|err| AppError::Database(err.to_string()))?,
    })
}

fn map_student_row(row: PgRow) -> Result<StudentRecordDto, AppError> {
    Ok(StudentRecordDto {
        id: row
            .try_get("id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        homeroom_id: row
            .try_get("homeroom_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        full_name: row
            .try_get("full_name")
            .map_err(|err| AppError::Database(err.to_string()))?,
        student_code: row
            .try_get("student_code")
            .map_err(|err| AppError::Database(err.to_string()))?,
        is_teacher_child: row
            .try_get("is_teacher_child")
            .map_err(|err| AppError::Database(err.to_string()))?,
        primary_guardian_name: row
            .try_get("primary_guardian_name")
            .map_err(|err| AppError::Database(err.to_string()))?,
        primary_guardian_phone: row
            .try_get("primary_guardian_phone")
            .map_err(|err| AppError::Database(err.to_string()))?,
        status: row
            .try_get("status")
            .map_err(|err| AppError::Database(err.to_string()))?,
    })
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

impl HomeroomUpdateChanges {
    fn has_updates(&self) -> bool {
        self.display_name.is_some()
            || self.grade_label.is_some()
            || self.class_label.is_some()
            || self.head_teacher_name.is_some()
            || self.head_teacher_phone.is_some()
            || self.notes.is_some()
    }
}

impl UpdateStudentChanges {
    fn has_updates(&self) -> bool {
        self.homeroom_id.is_some()
            || self.full_name.is_some()
            || self.student_code.is_some()
            || self.is_teacher_child.is_some()
            || self.primary_guardian_name.is_some()
            || self.primary_guardian_phone.is_some()
            || self.status.is_some()
    }
}

#[derive(Clone)]
struct IndexedStudent {
    id: Uuid,
    homeroom_id: Uuid,
    is_teacher_child: bool,
}

#[derive(Debug)]
struct TeacherChildDraft {
    row_number: u32,
    class_name: String,
    student_name: String,
    student_code: Option<String>,
}

#[derive(Debug)]
struct ParsedTeacherChildDrafts {
    drafts: Vec<TeacherChildDraft>,
    skipped_rows: i32,
}

impl TeacherChildImportSummary {
    fn new(total_rows: i32, skipped_rows: i32) -> Self {
        Self {
            total_rows,
            matched_students: 0,
            updated_students: 0,
            already_marked: 0,
            skipped_rows,
            duplicate_rows: 0,
            errors: Vec::new(),
        }
    }

    fn record_error(&mut self, row: u32, message: impl Into<String>) {
        self.errors.push(TeacherChildImportError {
            row,
            message: message.into(),
        });
    }
}

#[derive(Debug, Clone)]
struct TeacherChildImportColumns {
    mode: TeacherChildColumnMode,
}

#[derive(Debug, Clone)]
enum TeacherChildColumnMode {
    Split {
        class_column: usize,
        student_column: usize,
    },
    Combined {
        identifier_column: usize,
    },
}

impl TeacherChildImportColumns {
    fn from_json(raw: Option<&str>) -> Result<Self, AppError> {
        let Some(text) = raw.filter(|value| !value.trim().is_empty()) else {
            return Ok(Self::default_split());
        };
        let parsed: RawTeacherChildColumnConfig = serde_json::from_str(text)
            .map_err(|err| AppError::Validation(format!("列配置 JSON 解析失败: {}", err)))?;
        Self::from_raw(parsed)
    }

    fn from_raw(raw: RawTeacherChildColumnConfig) -> Result<Self, AppError> {
        let normalized_mode = raw.mode.as_ref().map(|mode| mode.to_ascii_uppercase());
        match normalized_mode.as_deref() {
            Some("COMBINED") => Self::build_combined(raw.combined_column),
            Some("SPLIT") | None => {
                if normalized_mode.is_none()
                    && raw.combined_column.is_some()
                    && raw.class_column.is_none()
                    && raw.student_column.is_none()
                {
                    Self::build_combined(raw.combined_column)
                } else {
                    Self::build_split(raw.class_column, raw.student_column)
                }
            }
            Some(other) => Err(AppError::Validation(format!(
                "列配置 mode `{}` 不支持，请使用 SPLIT 或 COMBINED",
                other
            ))),
        }
    }

    fn default_split() -> Self {
        let class_column = column_label_to_index("B").unwrap_or(1);
        let student_column = column_label_to_index("C").unwrap_or(2);
        Self {
            mode: TeacherChildColumnMode::Split {
                class_column,
                student_column,
            },
        }
    }

    fn build_split(
        class_ref: Option<ColumnRef>,
        student_ref: Option<ColumnRef>,
    ) -> Result<Self, AppError> {
        let default_class = column_label_to_index("B").unwrap_or(1);
        let default_student = column_label_to_index("C").unwrap_or(2);
        let class_column = class_ref
            .map(|column| column.to_index().map_err(|msg| AppError::Validation(msg)))
            .transpose()?
            .unwrap_or(default_class);
        let student_column = student_ref
            .map(|column| column.to_index().map_err(|msg| AppError::Validation(msg)))
            .transpose()?
            .unwrap_or(default_student);
        Ok(Self {
            mode: TeacherChildColumnMode::Split {
                class_column,
                student_column,
            },
        })
    }

    fn build_combined(column: Option<ColumnRef>) -> Result<Self, AppError> {
        let fallback = ColumnRef::Letter("E".into());
        let resolved = column.unwrap_or(fallback);
        let index = resolved
            .to_index()
            .map_err(|msg| AppError::Validation(msg))?;
        Ok(Self {
            mode: TeacherChildColumnMode::Combined {
                identifier_column: index,
            },
        })
    }

    fn mode(&self) -> &TeacherChildColumnMode {
        &self.mode
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTeacherChildColumnConfig {
    mode: Option<String>,
    class_column: Option<ColumnRef>,
    student_column: Option<ColumnRef>,
    combined_column: Option<ColumnRef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ColumnRef {
    Letter(String),
    Index(u32),
}

impl ColumnRef {
    fn to_index(&self) -> Result<usize, String> {
        match self {
            ColumnRef::Letter(letter) => column_label_to_index(letter).ok_or_else(|| {
                format!("无法解析列字母 `{}`，请使用 Excel 列名如 A、B...AA", letter)
            }),
            ColumnRef::Index(index) => {
                if *index == 0 {
                    Err("列序号必须从 1 开始".into())
                } else {
                    Ok((*index as usize) - 1)
                }
            }
        }
    }
}

fn parse_teacher_child_drafts(
    workbook: &ExcelWorkbook,
    columns: &TeacherChildImportColumns,
) -> ParsedTeacherChildDrafts {
    let sheet = workbook.primary_sheet();
    if sheet.rows.len() <= 1 {
        return ParsedTeacherChildDrafts {
            drafts: Vec::new(),
            skipped_rows: 0,
        };
    }

    let mut drafts = Vec::new();
    let mut skipped = 0;
    for (row_index, row) in sheet.rows.iter().enumerate().skip(1) {
        let row_number = (row_index + 1) as u32;
        match columns.mode() {
            TeacherChildColumnMode::Split {
                class_column,
                student_column,
            } => {
                let class_value = read_cell(row, *class_column);
                let student_value = read_cell(row, *student_column);
                if class_value.is_empty() && student_value.is_empty() {
                    skipped += 1;
                    continue;
                }
                let (student_code, cleaned_name) = extract_student_code(&student_value);
                drafts.push(TeacherChildDraft {
                    row_number,
                    class_name: class_value,
                    student_name: cleaned_name,
                    student_code,
                });
            }
            TeacherChildColumnMode::Combined { identifier_column } => {
                let identifier = read_cell(row, *identifier_column);
                if identifier.is_empty() {
                    skipped += 1;
                    continue;
                }
                let (student_code, remainder) = extract_student_code(&identifier);
                let split = split_homeroom_and_name(&remainder);
                let (class_value, student_value) = split
                    .map(|(class_name, student_name)| (class_name, student_name))
                    .unwrap_or_else(|| (String::new(), remainder.trim().to_string()));
                drafts.push(TeacherChildDraft {
                    row_number,
                    class_name: class_value,
                    student_name: student_value,
                    student_code,
                });
            }
        }
    }

    ParsedTeacherChildDrafts {
        drafts,
        skipped_rows: skipped,
    }
}

fn read_cell(row: &[String], index: usize) -> String {
    row.get(index)
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

fn student_name_key(homeroom_id: Uuid, name: &str) -> String {
    format!("{}::{}", homeroom_id, normalize_label(name))
}

fn student_code_key(homeroom_id: Uuid, code: &str) -> String {
    format!("{}::{}", homeroom_id, normalize_code(code))
}

fn normalize_label(value: &str) -> String {
    value.trim().to_lowercase()
}

fn normalize_code(value: &str) -> String {
    value.trim().to_uppercase()
}

fn column_label_to_index(label: &str) -> Option<usize> {
    let mut result = 0usize;
    for ch in label.chars() {
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        let value = (ch.to_ascii_uppercase() as u8 - b'A' + 1) as usize;
        result = result * 26 + value;
    }
    if result == 0 { None } else { Some(result - 1) }
}

fn extract_student_code(value: &str) -> (Option<String>, String) {
    let trimmed = value.trim();
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let code = rest[..end].trim();
            let remainder = rest[end + 1..].trim().to_string();
            if !code.is_empty() {
                return (Some(code.to_string()), remainder);
            }
            return (None, remainder);
        }
    }
    (None, trimmed.to_string())
}

fn split_homeroom_and_name(value: &str) -> Option<(String, String)> {
    let separators = [' ', '　', '-', '－', '_', ':', '：', '/', '|'];
    for sep in separators {
        if let Some(idx) = value.rfind(sep) {
            let (left, right) = value.split_at(idx);
            let homeroom = left.trim();
            let mut student = right.trim_start_matches(sep).trim();
            if student.is_empty() {
                student = right.trim();
            }
            if !homeroom.is_empty() && !student.is_empty() {
                return Some((homeroom.to_string(), student.to_string()));
            }
        }
    }

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut ascii_prefix = false;
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_ascii_alphanumeric() {
            ascii_prefix = true;
            continue;
        }
        if ascii_prefix {
            let (left, right) = trimmed.split_at(idx);
            let homeroom = left.trim();
            let student = right.trim();
            if !homeroom.is_empty() && !student.is_empty() {
                return Some((homeroom.to_string(), student.to_string()));
            }
        }
        break;
    }

    None
}
