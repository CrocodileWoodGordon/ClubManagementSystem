use std::collections::HashMap;

use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    db::DbPool,
    domain::FeeBreakdown,
    error::AppError,
    services::billing_service::{BillingService, FeeBreakdownDetail},
};

#[derive(Debug)]
pub struct ReportingService<'a> {
    pool: &'a DbPool,
}

#[derive(Debug, Serialize)]
pub struct HomeroomBillingReport {
    pub homeroom: HomeroomBillingInfo,
    pub students: Vec<StudentBillingBundle>,
}

#[derive(Debug, Serialize)]
pub struct HomeroomBillingInfo {
    pub id: Uuid,
    pub display_name: String,
    pub campus_name: String,
}

#[derive(Debug, Serialize)]
pub struct StudentBillingBundle {
    pub student_id: Uuid,
    pub student_name: String,
    pub student_code: Option<String>,
    pub rows: Vec<StudentBillingItem>,
}

#[derive(Debug, Serialize)]
pub struct StudentBillingItem {
    #[serde(flatten)]
    pub breakdown: FeeBreakdown,
    pub club_id: Uuid,
    pub club_name: String,
    pub class_code: Option<String>,
}

impl From<FeeBreakdownDetail> for StudentBillingItem {
    fn from(detail: FeeBreakdownDetail) -> Self {
        Self {
            breakdown: detail.breakdown,
            club_id: detail.club_id,
            club_name: detail.club_name,
            class_code: detail.class_code,
        }
    }
}

impl<'a> ReportingService<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn preview_settlement(&self, class_id: Uuid) -> Result<Vec<FeeBreakdown>, AppError> {
        BillingService::new(self.pool)
            .preview_by_class(class_id)
            .await
    }

    pub async fn preview_student_bill(
        &self,
        student_id: Uuid,
    ) -> Result<Vec<FeeBreakdown>, AppError> {
        BillingService::new(self.pool)
            .preview_by_student(student_id)
            .await
    }

    pub async fn preview_homeroom_bill(
        &self,
        homeroom_id: Uuid,
    ) -> Result<HomeroomBillingReport, AppError> {
        let homeroom = self.fetch_homeroom_context(homeroom_id).await?;
        let students = self.fetch_homeroom_students(homeroom_id).await?;

        let mut bundles: Vec<StudentBillingBundle> = students
            .into_iter()
            .map(|student| StudentBillingBundle {
                student_id: student.id,
                student_name: student.full_name,
                student_code: student.student_code,
                rows: Vec::new(),
            })
            .collect();

        let student_ids: Vec<Uuid> = bundles.iter().map(|bundle| bundle.student_id).collect();
        let mut index = HashMap::new();
        for (idx, bundle) in bundles.iter().enumerate() {
            index.insert(bundle.student_id, idx);
        }

        let details = BillingService::new(self.pool)
            .preview_by_students_bulk(student_ids, homeroom.term_id)
            .await?;

        for detail in details {
            if let Some(&idx) = index.get(&detail.breakdown.student_id) {
                bundles[idx].rows.push(detail.into());
            }
        }

        Ok(HomeroomBillingReport {
            homeroom: HomeroomBillingInfo {
                id: homeroom.id,
                display_name: homeroom.display_name,
                campus_name: homeroom.campus_name,
            },
            students: bundles,
        })
    }

    async fn fetch_homeroom_context(
        &self,
        homeroom_id: Uuid,
    ) -> Result<HomeroomContextRow, AppError> {
        sqlx::query_as::<_, HomeroomContextRow>(
            r#"
                SELECT h.id,
                       h.term_id,
                       h.display_name,
                       COALESCE(c.name, '未命名校区') AS campus_name
                FROM homerooms h
                LEFT JOIN campuses c ON c.id = h.campus_id
                WHERE h.id = $1
            "#,
        )
        .bind(homeroom_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?
        .ok_or_else(|| AppError::NotFound("未找到指定班级".into()))
    }

    async fn fetch_homeroom_students(
        &self,
        homeroom_id: Uuid,
    ) -> Result<Vec<HomeroomStudentRow>, AppError> {
        sqlx::query_as::<_, HomeroomStudentRow>(
            r#"
                SELECT s.id,
                       s.full_name,
                       s.student_code
                FROM students s
                WHERE s.homeroom_id = $1
                  AND s.status = 'ACTIVE'
                ORDER BY s.full_name ASC
            "#,
        )
        .bind(homeroom_id)
        .fetch_all(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))
    }
}

#[derive(Debug, FromRow)]
struct HomeroomContextRow {
    id: Uuid,
    term_id: Uuid,
    display_name: String,
    campus_name: String,
}

#[derive(Debug, FromRow)]
struct HomeroomStudentRow {
    id: Uuid,
    full_name: String,
    student_code: Option<String>,
}
