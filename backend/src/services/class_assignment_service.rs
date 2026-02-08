use chrono::NaiveTime;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::{db::DbPool, domain::ClassStatus, error::AppError};

#[derive(Debug)]
pub struct ClassAssignmentService<'a> {
    pool: &'a DbPool,
}

#[derive(Debug)]
pub struct ClassLookupFilters {
    pub term_id: Option<Uuid>,
    pub campus_id: Uuid,
    pub club_id: Uuid,
    pub weekday: u8,
}

#[derive(Debug)]
pub struct CreateClassInput {
    pub term_id: Option<Uuid>,
    pub campus_id: Uuid,
    pub club_id: Uuid,
    pub weekday: u8,
    pub class_code: String,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub location: Option<String>,
    pub capacity: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug)]
pub struct ClassAssignmentInput {
    pub term_id: Option<Uuid>,
    pub campus_id: Uuid,
    pub club_id: Uuid,
    pub weekday: u8,
    pub class_id: Option<Uuid>,
    pub enrollment_ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ClassSummary {
    pub id: Uuid,
    pub term_id: Uuid,
    pub campus_id: Uuid,
    pub club_id: Uuid,
    pub class_code: String,
    pub weekday: u8,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub location: Option<String>,
    pub capacity: Option<i32>,
    pub status: ClassStatus,
    pub notes: Option<String>,
    pub assigned_count: i64,
}

impl<'a> ClassAssignmentService<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn list_classes(
        &self,
        filters: &ClassLookupFilters,
    ) -> Result<Vec<ClassSummary>, AppError> {
        if !(1..=7).contains(&filters.weekday) {
            return Err(AppError::Validation(
                "weekday 需在 1-7 之间（1=周一，7=周日）".into(),
            ));
        }
        let term_id = self.resolve_term_id(filters.term_id).await?;
        let rows = sqlx::query(
            r#"
                SELECT c.id,
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
                       c.notes,
                       COALESCE(stats.assigned_count, 0) AS assigned_count
                FROM classes c
                LEFT JOIN (
                    SELECT class_id, COUNT(*)::bigint AS assigned_count
                    FROM enrollments
                    WHERE term_id = $1
                      AND class_id IS NOT NULL
                      AND status IN ('PENDING','ACTIVE')
                    GROUP BY class_id
                ) stats ON stats.class_id = c.id
                WHERE c.term_id = $1
                  AND c.campus_id = $2
                  AND c.club_id = $3
                  AND c.weekday = $4
                ORDER BY c.class_code ASC
            "#,
        )
        .bind(term_id)
        .bind(filters.campus_id)
        .bind(filters.club_id)
        .bind(i16::from(filters.weekday))
        .fetch_all(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        rows.into_iter()
            .map(map_class_summary)
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn create_class(&self, input: &CreateClassInput) -> Result<ClassSummary, AppError> {
        validate_weekday(input.weekday)?;
        if input.class_code.trim().is_empty() {
            return Err(AppError::Validation("班级名称不能为空".into()));
        }
        if input.start_time >= input.end_time {
            return Err(AppError::Validation("开始时间需早于结束时间".into()));
        }
        if let Some(capacity) = input.capacity {
            if capacity <= 0 {
                return Err(AppError::Validation("班级容量必须为正数".into()));
            }
        }

        let term_id = self.resolve_term_id(input.term_id).await?;
        let row = sqlx::query(
            r#"
                INSERT INTO classes (
                    term_id, campus_id, club_id, class_code, weekday,
                    start_time, end_time, location, capacity, notes
                )
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                RETURNING id, term_id, campus_id, club_id, class_code, weekday,
                          start_time, end_time, location, capacity, status, notes
            "#,
        )
        .bind(term_id)
        .bind(input.campus_id)
        .bind(input.club_id)
        .bind(input.class_code.trim())
        .bind(i16::from(input.weekday))
        .bind(input.start_time)
        .bind(input.end_time)
        .bind(input.location.as_ref())
        .bind(input.capacity)
        .bind(input.notes.as_ref())
        .fetch_one(self.pool)
        .await
        .map_err(|err| {
            if let sqlx::Error::Database(db_err) = &err {
                if db_err.code().as_deref() == Some("23505") {
                    return AppError::Conflict("班级编号已存在，请修改后重试".into());
                }
            }
            AppError::Database(err.to_string())
        })?;

        let mut summary = map_class_summary(row)?;
        summary.assigned_count = 0;
        Ok(summary)
    }

    pub async fn assign_enrollments(&self, input: &ClassAssignmentInput) -> Result<u64, AppError> {
        if input.enrollment_ids.is_empty() {
            return Err(AppError::Validation("请至少选择一名学生".into()));
        }
        validate_weekday(input.weekday)?;
        let term_id = self.resolve_term_id(input.term_id).await?;
        if let Some(class_id) = input.class_id {
            self.ensure_class_membership(
                class_id,
                term_id,
                input.campus_id,
                input.club_id,
                input.weekday,
            )
            .await?;
        }

        let result = sqlx::query(
            r#"
                UPDATE enrollments
                SET class_id = $1,
                    status = CASE
                        WHEN $1 IS NULL THEN 'PENDING'
                        ELSE 'ACTIVE'
                    END,
                    updated_at = now()
                WHERE term_id = $2
                  AND campus_id = $3
                  AND club_id = $4
                  AND requested_weekday = $5
                  AND status IN ('PENDING','ACTIVE')
                  AND id = ANY($6)
            "#,
        )
        .bind(input.class_id)
        .bind(term_id)
        .bind(input.campus_id)
        .bind(input.club_id)
        .bind(i16::from(input.weekday))
        .bind(&input.enrollment_ids)
        .execute(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let affected = result.rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound("未找到可更新的报名记录".into()));
        }
        Ok(affected)
    }

    async fn ensure_class_membership(
        &self,
        class_id: Uuid,
        term_id: Uuid,
        campus_id: Uuid,
        club_id: Uuid,
        weekday: u8,
    ) -> Result<(), AppError> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
                SELECT COUNT(*)
                FROM classes
                WHERE id = $1
                  AND term_id = $2
                  AND campus_id = $3
                  AND club_id = $4
                  AND weekday = $5
            "#,
        )
        .bind(class_id)
        .bind(term_id)
        .bind(campus_id)
        .bind(club_id)
        .bind(i16::from(weekday))
        .fetch_one(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        if exists == 0 {
            return Err(AppError::Validation(
                "班级不属于当前筛选条件，无法分配".into(),
            ));
        }
        Ok(())
    }

    async fn resolve_term_id(&self, provided: Option<Uuid>) -> Result<Uuid, AppError> {
        if let Some(id) = provided {
            return Ok(id);
        }

        let term_id = sqlx::query_scalar(
            r#"
                SELECT id
                FROM terms
                WHERE is_active = true
                ORDER BY enrollment_start DESC
                LIMIT 1
            "#,
        )
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        term_id.ok_or_else(|| AppError::Validation("未找到激活学期，请提供 term_id".into()))
    }
}

fn validate_weekday(weekday: u8) -> Result<(), AppError> {
    if (1..=7).contains(&weekday) {
        Ok(())
    } else {
        Err(AppError::Validation(
            "weekday 需在 1-7 之间（1=周一，7=周日）".into(),
        ))
    }
}

fn map_class_summary(row: PgRow) -> Result<ClassSummary, AppError> {
    let status_raw: String = row
        .try_get("status")
        .map_err(|err| AppError::Database(err.to_string()))?;
    Ok(ClassSummary {
        id: row
            .try_get("id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        term_id: row
            .try_get("term_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        campus_id: row
            .try_get("campus_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        club_id: row
            .try_get("club_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        class_code: row
            .try_get("class_code")
            .map_err(|err| AppError::Database(err.to_string()))?,
        weekday: row
            .try_get::<i16, _>("weekday")
            .map_err(|err| AppError::Database(err.to_string()))? as u8,
        start_time: row
            .try_get("start_time")
            .map_err(|err| AppError::Database(err.to_string()))?,
        end_time: row
            .try_get("end_time")
            .map_err(|err| AppError::Database(err.to_string()))?,
        location: row
            .try_get("location")
            .map_err(|err| AppError::Database(err.to_string()))?,
        capacity: row
            .try_get("capacity")
            .map_err(|err| AppError::Database(err.to_string()))?,
        status: ClassStatus::from_str(&status_raw),
        notes: row
            .try_get("notes")
            .map_err(|err| AppError::Database(err.to_string()))?,
        assigned_count: row
            .try_get("assigned_count")
            .map_err(|err| AppError::Database(err.to_string()))?,
    })
}
