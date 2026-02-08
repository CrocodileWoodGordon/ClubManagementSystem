use serde::Serialize;
use sqlx::{QueryBuilder, Row, postgres::PgRow};
use uuid::Uuid;

use crate::{db::DbPool, domain::EnrollmentStatus, error::AppError};

#[derive(Debug)]
pub struct EnrollmentService<'a> {
    pool: &'a DbPool,
}

#[derive(Debug, Default)]
pub struct EnrollmentFilters {
    pub term_id: Option<Uuid>,
    pub campus_id: Option<Uuid>,
    pub homeroom: Option<String>,
    pub club_name: Option<String>,
    pub weekday: Option<u8>,
    pub student_name: Option<String>,
}

#[derive(Debug, Default)]
pub struct EnrollmentSummaryFilters {
    pub term_id: Option<Uuid>,
    pub campus_id: Option<Uuid>,
}

#[derive(Debug)]
pub struct EnrollmentSlotFilters {
    pub term_id: Option<Uuid>,
    pub campus_id: Uuid,
    pub club_id: Uuid,
    pub weekday: u8,
}

#[derive(Debug, Serialize)]
pub struct PendingEnrollmentDto {
    pub enrollment_id: Uuid,
    pub student_id: Uuid,
    pub student_name: String,
    pub student_code: Option<String>,
    pub homeroom: String,
    pub campus_id: Uuid,
    pub campus_name: String,
    pub club_id: Uuid,
    pub club_name: String,
    pub requested_weekday: u8,
    pub status: EnrollmentStatus,
    pub class_id: Option<Uuid>,
    pub class_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EnrollmentSummaryRow {
    pub campus_id: Uuid,
    pub campus_name: String,
    pub club_id: Uuid,
    pub club_name: String,
    pub requested_weekday: u8,
    pub total: i64,
}

impl<'a> EnrollmentService<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn list_pending(
        &self,
        filters: &EnrollmentFilters,
    ) -> Result<Vec<PendingEnrollmentDto>, AppError> {
        let term_id = self.resolve_term_id(filters.term_id).await?;
        let mut builder = QueryBuilder::new(
            r#"
            SELECT e.id AS enrollment_id,
                   e.student_id,
                   e.status,
                   e.requested_weekday,
                   e.class_id,
                   s.full_name AS student_name,
                   s.student_code,
                   h.display_name AS homeroom,
                   cam.id AS campus_id,
                   cam.name AS campus_name,
                   c.id AS club_id,
                   c.name AS club_name,
                   cls.class_code
            FROM enrollments e
            INNER JOIN students s ON s.id = e.student_id
            INNER JOIN homerooms h ON h.id = s.homeroom_id
            INNER JOIN campuses cam ON cam.id = e.campus_id
            INNER JOIN clubs c ON c.id = e.club_id
            LEFT JOIN classes cls ON cls.id = e.class_id
            WHERE e.term_id = "#,
        );
        builder.push_bind(term_id);
        builder.push(" AND e.status = 'PENDING'");

        if let Some(campus_id) = filters.campus_id {
            builder.push(" AND e.campus_id = ").push_bind(campus_id);
        }
        if let Some(weekday) = filters.weekday {
            builder
                .push(" AND e.requested_weekday = ")
                .push_bind(i16::from(weekday));
        }
        if let Some(homeroom) = filters
            .homeroom
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            builder
                .push(" AND h.display_name ILIKE ")
                .push_bind(format!("%{}%", homeroom.trim()));
        }
        if let Some(club_name) = filters
            .club_name
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            builder
                .push(" AND c.name ILIKE ")
                .push_bind(format!("%{}%", club_name.trim()));
        }
        if let Some(student_name) = filters
            .student_name
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            builder
                .push(" AND s.full_name ILIKE ")
                .push_bind(format!("%{}%", student_name.trim()));
        }

        builder.push(" ORDER BY h.display_name, s.full_name, e.requested_weekday");

        let rows = builder
            .build()
            .fetch_all(self.pool)
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        rows.into_iter()
            .map(map_pending_row)
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn list_slot_details(
        &self,
        filters: &EnrollmentSlotFilters,
    ) -> Result<Vec<PendingEnrollmentDto>, AppError> {
        let term_id = self.resolve_term_id(filters.term_id).await?;
        let rows = sqlx::query(
            r#"
                SELECT e.id AS enrollment_id,
                       e.student_id,
                       e.status,
                       e.requested_weekday,
                       e.class_id,
                       s.full_name AS student_name,
                       s.student_code,
                       h.display_name AS homeroom,
                       cam.id AS campus_id,
                       cam.name AS campus_name,
                       c.id AS club_id,
                       c.name AS club_name,
                       cls.class_code
                FROM enrollments e
                INNER JOIN students s ON s.id = e.student_id
                INNER JOIN homerooms h ON h.id = s.homeroom_id
                INNER JOIN campuses cam ON cam.id = e.campus_id
                INNER JOIN clubs c ON c.id = e.club_id
                LEFT JOIN classes cls ON cls.id = e.class_id
                WHERE e.term_id = $1
                  AND e.campus_id = $2
                  AND e.club_id = $3
                  AND e.requested_weekday = $4
                  AND e.status IN ('PENDING','ACTIVE')
                ORDER BY h.display_name, s.full_name
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
            .map(map_pending_row)
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn pending_summary(
        &self,
        filters: &EnrollmentSummaryFilters,
    ) -> Result<Vec<EnrollmentSummaryRow>, AppError> {
        let term_id = self.resolve_term_id(filters.term_id).await?;
        let mut builder = QueryBuilder::new(
            r#"
            SELECT e.campus_id,
                   cam.name AS campus_name,
                   e.requested_weekday,
                   c.id AS club_id,
                   c.name AS club_name,
                   COUNT(*)::bigint AS total
            FROM enrollments e
            INNER JOIN campuses cam ON cam.id = e.campus_id
            INNER JOIN clubs c ON c.id = e.club_id
            WHERE e.term_id = "#,
        );
        builder.push_bind(term_id);
        builder.push(" AND e.status IN ('PENDING','ACTIVE')");

        if let Some(campus_id) = filters.campus_id {
            builder.push(" AND e.campus_id = ").push_bind(campus_id);
        }

        builder.push(
            " GROUP BY e.campus_id, cam.name, e.requested_weekday, c.id, c.name
              ORDER BY cam.name, c.name, e.requested_weekday",
        );

        let rows = builder
            .build()
            .fetch_all(self.pool)
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        let mut summary = Vec::with_capacity(rows.len());
        for row in rows {
            summary.push(EnrollmentSummaryRow {
                campus_id: row
                    .try_get("campus_id")
                    .map_err(|err| AppError::Database(err.to_string()))?,
                campus_name: row
                    .try_get("campus_name")
                    .map_err(|err| AppError::Database(err.to_string()))?,
                club_id: row
                    .try_get("club_id")
                    .map_err(|err| AppError::Database(err.to_string()))?,
                club_name: row
                    .try_get("club_name")
                    .map_err(|err| AppError::Database(err.to_string()))?,
                requested_weekday: row
                    .try_get::<i16, _>("requested_weekday")
                    .map_err(|err| AppError::Database(err.to_string()))?
                    as u8,
                total: row
                    .try_get("total")
                    .map_err(|err| AppError::Database(err.to_string()))?,
            });
        }

        Ok(summary)
    }

    pub async fn update_status_batch(
        &self,
        _enrollment_ids: &[Uuid],
        _status: EnrollmentStatus,
    ) -> Result<(), AppError> {
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

fn map_status(raw: &str) -> EnrollmentStatus {
    match raw {
        "ACTIVE" => EnrollmentStatus::Active,
        "DROPPED" => EnrollmentStatus::Dropped,
        "TRANSFERRED_OUT" => EnrollmentStatus::TransferredOut,
        "TRANSFERRED_IN" => EnrollmentStatus::TransferredIn,
        _ => EnrollmentStatus::Pending,
    }
}

fn map_pending_row(row: PgRow) -> Result<PendingEnrollmentDto, AppError> {
    let status: String = row
        .try_get("status")
        .map_err(|err| AppError::Database(err.to_string()))?;
    Ok(PendingEnrollmentDto {
        enrollment_id: row
            .try_get("enrollment_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        student_id: row
            .try_get("student_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        student_name: row
            .try_get("student_name")
            .map_err(|err| AppError::Database(err.to_string()))?,
        student_code: row
            .try_get("student_code")
            .map_err(|err| AppError::Database(err.to_string()))?,
        homeroom: row
            .try_get("homeroom")
            .map_err(|err| AppError::Database(err.to_string()))?,
        campus_id: row
            .try_get("campus_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        campus_name: row
            .try_get("campus_name")
            .map_err(|err| AppError::Database(err.to_string()))?,
        club_id: row
            .try_get("club_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        club_name: row
            .try_get("club_name")
            .map_err(|err| AppError::Database(err.to_string()))?,
        requested_weekday: row
            .try_get::<i16, _>("requested_weekday")
            .map_err(|err| AppError::Database(err.to_string()))? as u8,
        status: map_status(&status),
        class_id: row
            .try_get("class_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        class_code: row
            .try_get("class_code")
            .map_err(|err| AppError::Database(err.to_string()))?,
    })
}
