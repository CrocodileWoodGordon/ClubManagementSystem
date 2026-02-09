use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{
    PgConnection, QueryBuilder, Row,
    postgres::{PgQueryResult, PgRow},
    types::BigDecimal,
};
use uuid::Uuid;

use crate::{db::DbPool, domain::EnrollmentStatus, error::AppError};

#[derive(Debug)]
pub struct ClubService<'a> {
    pool: &'a DbPool,
}

#[derive(Debug, Default)]
pub struct ClubListFilters {
    pub search: Option<String>,
    pub term_id: Option<Uuid>,
}

#[derive(Debug)]
pub struct NewClubInput {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub material_fee: f64,
    pub price_per_session: f64,
    pub grace_sessions: i16,
}

#[derive(Debug, Default)]
pub struct ClubUpdateChanges {
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub material_fee: Option<f64>,
    pub price_per_session: Option<f64>,
    pub grace_sessions: Option<i16>,
}

impl ClubUpdateChanges {
    pub fn has_changes(&self) -> bool {
        self.code.is_some()
            || self.name.is_some()
            || self.description.is_some()
            || self.material_fee.is_some()
            || self.price_per_session.is_some()
            || self.grace_sessions.is_some()
    }
}

#[derive(Debug)]
pub struct ClubMemberFilters {
    pub term_id: Uuid,
    pub campus_id: Uuid,
    pub weekday: Option<u8>,
}

#[derive(Debug)]
pub struct MembershipEntry {
    pub student_id: Uuid,
    pub requested_weekday: u8,
}

#[derive(Debug)]
pub struct AddMembersRequest {
    pub term_id: Uuid,
    pub campus_id: Uuid,
    pub entries: Vec<MembershipEntry>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ClubDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub material_fee: f64,
    pub price_per_session: f64,
    pub grace_sessions: i16,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placements: Vec<ClubPlacementDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClubPlacementDto {
    pub campus_id: Uuid,
    pub campus_name: String,
    pub weekday: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClubMemberDto {
    pub enrollment_id: Uuid,
    pub student_id: Uuid,
    pub student_name: String,
    pub student_code: Option<String>,
    pub homeroom: String,
    pub campus_id: Uuid,
    pub campus_name: String,
    pub term_id: Uuid,
    pub requested_weekday: u8,
    pub status: EnrollmentStatus,
}

#[derive(Debug, sqlx::FromRow)]
struct ClubRecord {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub material_fee: BigDecimal,
    pub price_per_session: BigDecimal,
    pub grace_sessions: i16,
    pub created_at: DateTime<Utc>,
}

impl From<ClubRecord> for ClubDto {
    fn from(record: ClubRecord) -> Self {
        let ClubRecord {
            id,
            code,
            name,
            description,
            material_fee,
            price_per_session,
            grace_sessions,
            created_at,
        } = record;
        Self {
            id,
            code,
            name,
            description,
            material_fee: decimal_to_f64(material_fee),
            price_per_session: decimal_to_f64(price_per_session),
            grace_sessions,
            created_at,
            placements: Vec::new(),
        }
    }
}

fn decimal_to_f64(value: BigDecimal) -> f64 {
    value
        .to_string()
        .parse::<f64>()
        .unwrap_or(0.0)
}

impl<'a> ClubService<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, filters: &ClubListFilters) -> Result<Vec<ClubDto>, AppError> {
        let mut builder = QueryBuilder::new(
            r#"
            SELECT id,
                   code,
                   name,
                   description,
                   material_fee,
                   price_per_session,
                   grace_sessions,
                   created_at
            FROM clubs
            WHERE 1 = 1
        "#,
        );

        if let Some(search) = filters
            .search
            .as_ref()
            .and_then(|value| non_empty(value).map(|v| v.to_string()))
        {
            let like = format!("%{}%", search);
            builder
                .push(" AND (code ILIKE ")
                .push_bind(like.clone())
                .push(" OR name ILIKE ")
                .push_bind(like)
                .push(")");
        }

        builder.push(" ORDER BY name ASC");

        let records = builder
            .build_query_as::<ClubRecord>()
            .fetch_all(self.pool)
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        let mut clubs: Vec<ClubDto> = records.into_iter().map(ClubDto::from).collect();

        if clubs.is_empty() {
            return Ok(clubs);
        }

        let term_id = self.resolve_term_id(filters.term_id).await?;
        let placement_map = self.load_placements(term_id).await?;
        for club in &mut clubs {
            if let Some(items) = placement_map.get(&club.id) {
                club.placements = items.clone();
            }
        }

        Ok(clubs)
    }

    pub async fn create(&self, input: NewClubInput) -> Result<ClubDto, AppError> {
        validate_money(input.material_fee, "material_fee")?;
        validate_money(input.price_per_session, "price_per_session")?;
        if input.grace_sessions < 0 {
            return Err(AppError::Validation("grace_sessions 需为非负整数".into()));
        }

        let NewClubInput {
            code,
            name,
            description,
            material_fee,
            price_per_session,
            grace_sessions,
        } = input;
        let code = require_text("社团编码", code)?;
        let name = require_text("社团名称", name)?;
        let description = description.and_then(|value| non_empty(&value).map(|v| v.to_string()));

        sqlx::query_as::<_, ClubRecord>(
            r#"
                INSERT INTO clubs (code, name, description, material_fee, price_per_session, grace_sessions)
                VALUES ($1,$2,$3,$4,$5,$6)
                RETURNING id,
                          code,
                          name,
                          description,
                          material_fee,
                          price_per_session,
                          grace_sessions,
                          created_at
            "#,
        )
        .bind(code)
        .bind(name)
        .bind(description)
        .bind(material_fee)
        .bind(price_per_session)
        .bind(grace_sessions)
        .fetch_one(self.pool)
        .await
        .map(ClubDto::from)
        .map_err(|err| AppError::Database(err.to_string()))
    }

    pub async fn update(
        &self,
        club_id: Uuid,
        changes: ClubUpdateChanges,
    ) -> Result<ClubDto, AppError> {
        if !changes.has_changes() {
            return Err(AppError::Validation("请至少提供一个需要更新的字段".into()));
        }

        let ClubUpdateChanges {
            code,
            name,
            description,
            material_fee,
            price_per_session,
            grace_sessions,
        } = changes;

        let code = match code {
            Some(value) => Some(require_text("社团编码", value)?),
            None => None,
        };
        let name = match name {
            Some(value) => Some(require_text("社团名称", value)?),
            None => None,
        };
        if let Some(value) = material_fee {
            validate_money(value, "material_fee")?;
        }
        if let Some(value) = price_per_session {
            validate_money(value, "price_per_session")?;
        }
        if let Some(value) = grace_sessions {
            if value < 0 {
                return Err(AppError::Validation("grace_sessions 需为非负整数".into()));
            }
        }
        let description = description.and_then(|value| non_empty(&value).map(|v| v.to_string()));

        sqlx::query_as::<_, ClubRecord>(
            r#"
                UPDATE clubs
                SET code = COALESCE($2, code),
                    name = COALESCE($3, name),
                    description = COALESCE($4, description),
                    material_fee = COALESCE($5, material_fee),
                    price_per_session = COALESCE($6, price_per_session),
                    grace_sessions = COALESCE($7, grace_sessions)
                WHERE id = $1
                RETURNING id,
                          code,
                          name,
                          description,
                          material_fee,
                          price_per_session,
                          grace_sessions,
                          created_at
            "#,
        )
        .bind(club_id)
        .bind(code)
        .bind(name)
        .bind(description)
        .bind(material_fee)
        .bind(price_per_session)
        .bind(grace_sessions)
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?
        .map(|record| record.into())
        .ok_or_else(|| AppError::NotFound("未找到指定社团".into()))
    }

    pub async fn delete(&self, club_id: Uuid) -> Result<(), AppError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        delete_enrollments(club_id, tx.as_mut()).await?;
        delete_classes(club_id, tx.as_mut()).await?;

        let result = sqlx::query("DELETE FROM clubs WHERE id = $1")
            .bind(club_id)
            .execute(tx.as_mut())
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        if result.rows_affected() == 0 {
            tx.rollback()
                .await
                .map_err(|err| AppError::Database(err.to_string()))?;
            return Err(AppError::NotFound("未找到指定社团".into()));
        }

        tx.commit()
            .await
            .map_err(|err| AppError::Database(err.to_string()))
    }

    pub async fn list_members(
        &self,
        club_id: Uuid,
        filters: &ClubMemberFilters,
    ) -> Result<Vec<ClubMemberDto>, AppError> {
        let mut builder = QueryBuilder::new(
            r#"
            SELECT e.id AS enrollment_id,
                   e.student_id,
                   s.full_name AS student_name,
                   s.student_code,
                   h.display_name AS homeroom,
                   e.campus_id,
                   cam.name AS campus_name,
                   e.term_id,
                   e.requested_weekday,
                   e.status
            FROM enrollments e
            INNER JOIN students s ON s.id = e.student_id
            INNER JOIN homerooms h ON h.id = s.homeroom_id
            INNER JOIN campuses cam ON cam.id = e.campus_id
            WHERE e.club_id = "#,
        );
        builder.push_bind(club_id);
        builder.push(" AND e.term_id = ").push_bind(filters.term_id);
        builder
            .push(" AND e.campus_id = ")
            .push_bind(filters.campus_id);
        builder.push(" AND e.status IN ('PENDING','ACTIVE')");
        if let Some(weekday) = filters.weekday {
            builder
                .push(" AND e.requested_weekday = ")
                .push_bind(i16::from(weekday));
        }
        builder.push(" ORDER BY h.display_name, s.full_name");

        let rows = builder
            .build()
            .fetch_all(self.pool)
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        rows.into_iter()
            .map(map_member_row)
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn add_members(
        &self,
        club_id: Uuid,
        request: AddMembersRequest,
    ) -> Result<Vec<ClubMemberDto>, AppError> {
        if request.entries.is_empty() {
            return Err(AppError::Validation("请至少选择一名学生".into()));
        }
        validate_weekdays(&request.entries)?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        ensure_club_exists(club_id, tx.as_mut()).await?;
        ensure_term_exists(request.term_id, tx.as_mut()).await?;
        ensure_campus_exists(request.campus_id, tx.as_mut()).await?;
        ensure_students_in_context(&request, tx.as_mut()).await?;

        let mut inserted_ids = Vec::with_capacity(request.entries.len());
        for entry in &request.entries {
            ensure_membership_absent(
                club_id,
                request.term_id,
                request.campus_id,
                entry,
                tx.as_mut(),
            )
            .await?;

            let inserted = sqlx::query_scalar(
                r#"
                    INSERT INTO enrollments (term_id, campus_id, student_id, club_id, requested_weekday)
                    VALUES ($1,$2,$3,$4,$5)
                    RETURNING id
                "#,
            )
            .bind(request.term_id)
            .bind(request.campus_id)
            .bind(entry.student_id)
            .bind(club_id)
            .bind(i16::from(entry.requested_weekday))
            .fetch_one(tx.as_mut())
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

            inserted_ids.push(inserted);
        }

        tx.commit()
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        self.fetch_members_by_ids(&inserted_ids).await
    }

    pub async fn remove_member(&self, club_id: Uuid, enrollment_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
                UPDATE enrollments
                SET status = 'DROPPED',
                    drop_date = CURRENT_DATE,
                    status_reason = '社团管理页面移除',
                    class_id = NULL
                WHERE id = $1
                  AND club_id = $2
                  AND status IN ('PENDING','ACTIVE')
            "#,
        )
        .bind(enrollment_id)
        .bind(club_id)
        .execute(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("未找到可移除的成员".into()));
        }

        Ok(())
    }

    async fn fetch_members_by_ids(
        &self,
        enrollment_ids: &[Uuid],
    ) -> Result<Vec<ClubMemberDto>, AppError> {
        if enrollment_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut builder = QueryBuilder::new(
            r#"
            SELECT e.id AS enrollment_id,
                   e.student_id,
                   s.full_name AS student_name,
                   s.student_code,
                   h.display_name AS homeroom,
                   e.campus_id,
                   cam.name AS campus_name,
                   e.term_id,
                   e.requested_weekday,
                   e.status
            FROM enrollments e
            INNER JOIN students s ON s.id = e.student_id
            INNER JOIN homerooms h ON h.id = s.homeroom_id
            INNER JOIN campuses cam ON cam.id = e.campus_id
            WHERE e.id IN (
        "#,
        );
        {
            let mut separated = builder.separated(", ");
            for id in enrollment_ids {
                separated.push_bind(id);
            }
        }
        builder.push(") ORDER BY h.display_name, s.full_name");

        let rows = builder
            .build()
            .fetch_all(self.pool)
            .await
            .map_err(|err| AppError::Database(err.to_string()))?;

        rows.into_iter()
            .map(map_member_row)
            .collect::<Result<Vec<_>, _>>()
    }
}

impl<'a> ClubService<'a> {
    async fn load_placements(
        &self,
        term_id: Uuid,
    ) -> Result<HashMap<Uuid, Vec<ClubPlacementDto>>, AppError> {
        let rows = sqlx::query(
            r#"
                SELECT DISTINCT e.club_id,
                                e.campus_id,
                                cam.name AS campus_name,
                                e.requested_weekday AS weekday
                FROM enrollments e
                INNER JOIN campuses cam ON cam.id = e.campus_id
                WHERE e.term_id = $1
                  AND e.status IN ('PENDING','ACTIVE')
            "#,
        )
        .bind(term_id)
        .fetch_all(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        let mut map: HashMap<Uuid, Vec<ClubPlacementDto>> = HashMap::new();
        for row in rows {
            let club_id: Uuid = row
                .try_get("club_id")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let campus_id: Uuid = row
                .try_get("campus_id")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let campus_name: String = row
                .try_get("campus_name")
                .map_err(|err| AppError::Database(err.to_string()))?;
            let weekday: i16 = row
                .try_get("weekday")
                .map_err(|err| AppError::Database(err.to_string()))?;

            map.entry(club_id).or_default().push(ClubPlacementDto {
                campus_id,
                campus_name,
                weekday: weekday as u8,
            });
        }

        for placements in map.values_mut() {
            placements.sort_by(|a, b| {
                a.campus_name
                    .cmp(&b.campus_name)
                    .then(a.weekday.cmp(&b.weekday))
            });
        }

        Ok(map)
    }

    async fn resolve_term_id(&self, provided: Option<Uuid>) -> Result<Uuid, AppError> {
        if let Some(id) = provided {
            return Ok(id);
        }
        let id = sqlx::query_scalar(
            r#"
                SELECT id
                FROM terms
                WHERE is_active = true
                ORDER BY start_date DESC
                LIMIT 1
            "#,
        )
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;
        id.ok_or_else(|| AppError::Validation("未找到激活学期，请提供 term_id".into()))
    }
}

async fn delete_enrollments(
    club_id: Uuid,
    conn: &mut PgConnection,
) -> Result<PgQueryResult, AppError> {
    sqlx::query("DELETE FROM enrollments WHERE club_id = $1")
        .bind(club_id)
        .execute(conn)
        .await
        .map_err(|err| AppError::Database(err.to_string()))
}

async fn delete_classes(club_id: Uuid, conn: &mut PgConnection) -> Result<PgQueryResult, AppError> {
    sqlx::query("DELETE FROM classes WHERE club_id = $1")
        .bind(club_id)
        .execute(conn)
        .await
        .map_err(|err| AppError::Database(err.to_string()))
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn require_text(field: &str, value: String) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(AppError::Validation(format!("{}不能为空", field)))
    } else {
        Ok(trimmed.to_string())
    }
}

fn validate_money(value: f64, field: &str) -> Result<(), AppError> {
    if value < 0.0 {
        return Err(AppError::Validation(format!("{} 需为非负数", field)));
    }
    Ok(())
}

fn map_member_row(row: PgRow) -> Result<ClubMemberDto, AppError> {
    let status: String = row
        .try_get("status")
        .map_err(|err| AppError::Database(err.to_string()))?;

    Ok(ClubMemberDto {
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
        term_id: row
            .try_get("term_id")
            .map_err(|err| AppError::Database(err.to_string()))?,
        requested_weekday: row
            .try_get::<i16, _>("requested_weekday")
            .map_err(|err| AppError::Database(err.to_string()))? as u8,
        status: map_status(&status),
    })
}

fn map_status(value: &str) -> EnrollmentStatus {
    match value {
        "ACTIVE" => EnrollmentStatus::Active,
        "DROPPED" => EnrollmentStatus::Dropped,
        "TRANSFERRED_OUT" => EnrollmentStatus::TransferredOut,
        "TRANSFERRED_IN" => EnrollmentStatus::TransferredIn,
        _ => EnrollmentStatus::Pending,
    }
}

fn validate_weekdays(entries: &[MembershipEntry]) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for entry in entries {
        if entry.requested_weekday == 0 || entry.requested_weekday > 7 {
            return Err(AppError::Validation("星期需在 1-7 之间（1=周一）".into()));
        }
        if !seen.insert((entry.student_id, entry.requested_weekday)) {
            return Err(AppError::Validation(
                "同一学生在同一天重复添加，请检查录入".into(),
            ));
        }
    }
    Ok(())
}

async fn ensure_club_exists(club_id: Uuid, conn: &mut PgConnection) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, i64>("SELECT 1 FROM clubs WHERE id = $1")
        .bind(club_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;
    if exists.is_none() {
        return Err(AppError::NotFound("未找到指定社团".into()));
    }
    Ok(())
}

async fn ensure_term_exists(term_id: Uuid, conn: &mut PgConnection) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, i64>("SELECT 1 FROM terms WHERE id = $1")
        .bind(term_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;
    if exists.is_none() {
        return Err(AppError::Validation("term_id 无效".into()));
    }
    Ok(())
}

async fn ensure_campus_exists(campus_id: Uuid, conn: &mut PgConnection) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, i64>("SELECT 1 FROM campuses WHERE id = $1")
        .bind(campus_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;
    if exists.is_none() {
        return Err(AppError::Validation("campus_id 无效".into()));
    }
    Ok(())
}

async fn ensure_students_in_context(
    request: &AddMembersRequest,
    conn: &mut PgConnection,
) -> Result<(), AppError> {
    let mut unique_ids = HashSet::new();
    let mut ordered_ids = Vec::new();
    for entry in &request.entries {
        if unique_ids.insert(entry.student_id) {
            ordered_ids.push(entry.student_id);
        }
    }

    let mut builder = QueryBuilder::new(
        r#"
        SELECT s.id
        FROM students s
        INNER JOIN homerooms h ON h.id = s.homeroom_id
        WHERE s.status = 'ACTIVE'
          AND h.term_id = "#,
    );
    builder.push_bind(request.term_id);
    builder
        .push(" AND h.campus_id = ")
        .push_bind(request.campus_id);
    builder.push(" AND s.id IN (");
    {
        let mut separated = builder.separated(", ");
        for id in &ordered_ids {
            separated.push_bind(id);
        }
    }
    builder.push(")");

    let rows = builder
        .build()
        .fetch_all(&mut *conn)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

    let mut found = HashSet::new();
    for row in rows {
        let student_id: Uuid = row
            .try_get("id")
            .map_err(|err| AppError::Database(err.to_string()))?;
        found.insert(student_id);
    }

    for entry in &request.entries {
        if !found.contains(&entry.student_id) {
            return Err(AppError::Validation(format!(
                "学生 {:?} 不属于当前学期/校区或状态非 ACTIVE",
                entry.student_id
            )));
        }
    }

    Ok(())
}

async fn ensure_membership_absent(
    club_id: Uuid,
    term_id: Uuid,
    campus_id: Uuid,
    entry: &MembershipEntry,
    conn: &mut PgConnection,
) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, Uuid>(
        r#"
            SELECT id
            FROM enrollments
            WHERE club_id = $1
              AND term_id = $2
              AND campus_id = $3
              AND student_id = $4
              AND requested_weekday = $5
              AND status IN ('PENDING','ACTIVE')
        "#,
    )
    .bind(club_id)
    .bind(term_id)
    .bind(campus_id)
    .bind(entry.student_id)
    .bind(i16::from(entry.requested_weekday))
    .fetch_optional(&mut *conn)
    .await
    .map_err(|err| AppError::Database(err.to_string()))?;

    if exists.is_some() {
        return Err(AppError::Conflict(format!(
            "学生 {:?} 在周{} 已报名该社团",
            entry.student_id, entry.requested_weekday
        )));
    }

    Ok(())
}
