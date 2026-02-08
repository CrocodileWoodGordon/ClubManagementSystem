use std::collections::HashMap;

use chrono::Datelike;
use serde::Serialize;
use sqlx::{QueryBuilder, Row, postgres::PgRow};
use uuid::Uuid;

use crate::{db::DbPool, error::AppError};

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
