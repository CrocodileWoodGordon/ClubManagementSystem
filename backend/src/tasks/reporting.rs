#![allow(dead_code)]

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    db::DbPool,
    domain::{
        BillingPolicySnapshot, BillingRun, BillingRunStatus, BillingRunType, FeeBreakdown,
        MaterialChargeReason, TuitionWaiverReason,
    },
    error::AppError,
    services::BillingService,
};

const CSV_HEADER: &str = "term_id,class_id,enrollment_id,student_id,lesson_fee,material_fee,discount_amount,charged_sessions,attendance_count,waive_reason,remarks\n";

#[derive(Debug, Clone)]
pub struct ReportFile {
    pub file_name: String,
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SettlementBatchResult {
    pub run: BillingRun,
    pub total_enrollments: usize,
    pub total_items: usize,
    pub class_count: usize,
    pub file: ReportFile,
}

pub struct ReportingTask<'a> {
    pool: &'a DbPool,
}

impl<'a> ReportingTask<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Run a billing batch for the given term and export the CSV report.
    pub async fn run_settlement_batch(
        &self,
        term_id: Uuid,
        triggered_by: &str,
        run_type: BillingRunType,
    ) -> Result<SettlementBatchResult, AppError> {
        let created_run = self.insert_run(term_id, run_type, triggered_by).await?;

        let collected = match self.collect_fee_breakdowns(term_id).await {
            Ok(data) => data,
            Err(err) => {
                let _ = self
                    .update_run_status(
                        created_run.id,
                        BillingRunStatus::Failed,
                        Some(err.to_string()),
                    )
                    .await;
                return Err(err);
            }
        };

        let total_items = match self
            .persist_billing_items(created_run.id, &collected.entries)
            .await
        {
            Ok(count) => count,
            Err(err) => {
                let _ = self
                    .update_run_status(
                        created_run.id,
                        BillingRunStatus::Failed,
                        Some(err.to_string()),
                    )
                    .await;
                return Err(err);
            }
        };

        let file = build_csv(term_id, &collected.entries);
        let class_count = collected.class_count;
        let enrollment_count = collected
            .entries
            .iter()
            .map(|entry| entry.enrollment_id)
            .collect::<HashSet<_>>()
            .len();

        let summary_note = format!(
            "{} classes, {} enrollments, {} billing items",
            class_count, enrollment_count, total_items
        );

        let finished_run = self
            .update_run_status(
                created_run.id,
                BillingRunStatus::Completed,
                Some(summary_note),
            )
            .await?;

        Ok(SettlementBatchResult {
            run: finished_run,
            total_enrollments: enrollment_count,
            total_items,
            class_count,
            file,
        })
    }

    async fn insert_run(
        &self,
        term_id: Uuid,
        run_type: BillingRunType,
        triggered_by: &str,
    ) -> Result<BillingRun, AppError> {
        let row = sqlx::query_as::<_, BillingRunRow>(
            r#"
            INSERT INTO billing_runs (term_id, run_type, status, triggered_by)
            VALUES ($1, $2, 'RUNNING', $3)
            RETURNING id, term_id, run_type, status, triggered_by, started_at, completed_at, notes
        "#,
        )
        .bind(term_id)
        .bind(run_type_to_db(run_type))
        .bind(triggered_by)
        .fetch_one(self.pool)
        .await
        .map_err(map_db_error)?;

        Ok(row.into())
    }

    async fn update_run_status(
        &self,
        run_id: Uuid,
        status: BillingRunStatus,
        notes: Option<String>,
    ) -> Result<BillingRun, AppError> {
        let row = sqlx::query_as::<_, BillingRunRow>(
            r#"
            UPDATE billing_runs
            SET status = $2,
                notes = $3,
                completed_at = CASE
                    WHEN $2 IN ('COMPLETED','FAILED') THEN now()
                    ELSE completed_at
                END
            WHERE id = $1
            RETURNING id, term_id, run_type, status, triggered_by, started_at, completed_at, notes
        "#,
        )
        .bind(run_id)
        .bind(status_to_db(status))
        .bind(notes)
        .fetch_one(self.pool)
        .await
        .map_err(map_db_error)?;

        Ok(row.into())
    }

    async fn collect_fee_breakdowns(&self, term_id: Uuid) -> Result<SettlementData, AppError> {
        let class_ids = sqlx::query_scalar(
            r#"
                SELECT id
                FROM classes
                WHERE term_id = $1
                ORDER BY weekday ASC, class_code ASC
            "#,
        )
        .bind(term_id)
        .fetch_all(self.pool)
        .await
        .map_err(map_db_error)?;

        let total_classes = class_ids.len();
        let service = BillingService::new(self.pool);
        let mut entries = Vec::new();
        for class_id in class_ids.iter().copied() {
            let mut class_entries = service.preview_by_class(class_id).await?;
            entries.append(&mut class_entries);
        }

        Ok(SettlementData {
            entries,
            class_count: total_classes,
        })
    }

    async fn persist_billing_items(
        &self,
        run_id: Uuid,
        entries: &[FeeBreakdown],
    ) -> Result<usize, AppError> {
        if entries.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let mut inserted = 0usize;
        for entry in entries {
            for item in build_items_from_breakdown(entry) {
                let snapshot: Value = serde_json::to_value(&item.policy_snapshot)
                    .map_err(|err| AppError::Unknown(err.to_string()))?;

                sqlx::query(
                    r#"
                    INSERT INTO billing_items (
                        billing_run_id,
                        enrollment_id,
                        item_type,
                        quantity,
                        unit_amount,
                        total_amount,
                        source_attendance,
                        policy_snapshot,
                        note
                    )
                    VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                "#,
                )
                .bind(run_id)
                .bind(item.enrollment_id)
                .bind(item.item_type)
                .bind(item.quantity)
                .bind(item.unit_amount)
                .bind(item.total_amount)
                .bind(item.source_attendance)
                .bind(snapshot)
                .bind(item.note)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;

                inserted += 1;
            }
        }

        tx.commit().await.map_err(map_db_error)?;
        Ok(inserted)
    }
}

struct SettlementData {
    entries: Vec<FeeBreakdown>,
    class_count: usize,
}

#[derive(Debug, FromRow)]
struct BillingRunRow {
    id: Uuid,
    term_id: Uuid,
    run_type: String,
    status: String,
    triggered_by: Option<String>,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    notes: Option<String>,
}

impl From<BillingRunRow> for BillingRun {
    fn from(row: BillingRunRow) -> Self {
        BillingRun {
            id: row.id,
            term_id: row.term_id,
            run_type: parse_run_type(&row.run_type),
            status: parse_run_status(&row.status),
            triggered_by: row.triggered_by.unwrap_or_default(),
            started_at: Some(row.started_at),
            completed_at: row.completed_at,
            notes: row.notes,
        }
    }
}

fn build_csv(term_id: Uuid, entries: &[FeeBreakdown]) -> ReportFile {
    let mut buffer = String::with_capacity(entries.len() * 128 + CSV_HEADER.len());
    buffer.push_str(CSV_HEADER);
    for entry in entries {
        let columns = [
            term_id.to_string(),
            entry.class_id.to_string(),
            entry.enrollment_id.to_string(),
            entry.student_id.to_string(),
            format_amount(entry.lesson_fee),
            format_amount(entry.material_fee),
            format_amount(entry.discount_amount),
            entry.charged_sessions.to_string(),
            entry.attendance_count.to_string(),
            waiver_to_string(entry.waive_reason),
            entry.remarks.clone().unwrap_or_default(),
        ];
        let escaped = columns
            .iter()
            .map(|value| escape_csv(value))
            .collect::<Vec<_>>()
            .join(",");
        buffer.push_str(&escaped);
        buffer.push('\n');
    }

    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let file_name = format!("settlement_{}_{}.csv", term_id, timestamp);
    ReportFile {
        file_name,
        mime_type: "text/csv".to_string(),
        bytes: buffer.into_bytes(),
    }
}

fn escape_csv(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        let escaped = value.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        value.to_string()
    }
}

fn format_amount(value: f64) -> String {
    format!("{value:.2}")
}

fn waiver_to_string(reason: Option<TuitionWaiverReason>) -> String {
    match reason {
        Some(TuitionWaiverReason::DropWithinGrace) => "DROP_WITHIN_GRACE".to_string(),
        Some(TuitionWaiverReason::ManualOverride) => "MANUAL_OVERRIDE".to_string(),
        Some(TuitionWaiverReason::TeacherBenefit) => "TEACHER_BENEFIT".to_string(),
        None => String::new(),
    }
}

struct BillingItemInsert {
    enrollment_id: Uuid,
    item_type: &'static str,
    quantity: f64,
    unit_amount: f64,
    total_amount: f64,
    source_attendance: Option<i32>,
    policy_snapshot: BillingPolicySnapshot,
    note: Option<String>,
}

fn build_items_from_breakdown(entry: &FeeBreakdown) -> Vec<BillingItemInsert> {
    let mut items = Vec::new();

    if !nearly_zero(entry.lesson_fee) || entry.charged_sessions > 0 {
        let quantity = entry.charged_sessions as f64;
        let unit_amount = if quantity > 0.0 {
            entry.lesson_fee / quantity
        } else {
            0.0
        };

        let mut snapshot = BillingPolicySnapshot {
            tuition_grace_applied: matches!(
                entry.waive_reason,
                Some(TuitionWaiverReason::DropWithinGrace)
            ),
            waived_tuition_reason: entry.waive_reason,
            ..Default::default()
        };

        if matches!(
            entry.waive_reason,
            Some(TuitionWaiverReason::TeacherBenefit)
        ) {
            snapshot.is_teacher_child = true;
            snapshot.teacher_discount_rate = Some(1.0);
        }

        items.push(BillingItemInsert {
            enrollment_id: entry.enrollment_id,
            item_type: "TUITION",
            quantity,
            unit_amount,
            total_amount: entry.lesson_fee,
            source_attendance: Some(clamp_to_i32(entry.attendance_count)),
            policy_snapshot: snapshot,
            note: note_from_breakdown(entry),
        });
    }

    if !nearly_zero(entry.material_fee) {
        let reason = infer_material_reason(entry.material_fee);
        let mut snapshot = BillingPolicySnapshot::default();
        snapshot.material_charge_reason = Some(reason);

        items.push(BillingItemInsert {
            enrollment_id: entry.enrollment_id,
            item_type: "MATERIAL",
            quantity: 1.0,
            unit_amount: entry.material_fee,
            total_amount: entry.material_fee,
            source_attendance: None,
            policy_snapshot: snapshot,
            note: entry.remarks.clone(),
        });
    }

    items
}

fn infer_material_reason(amount: f64) -> MaterialChargeReason {
    if amount > 0.0 {
        MaterialChargeReason::ChargeOnce
    } else if amount < 0.0 {
        MaterialChargeReason::Refunded
    } else {
        MaterialChargeReason::AlreadyCharged
    }
}

fn note_from_breakdown(entry: &FeeBreakdown) -> Option<String> {
    let mut parts = Vec::new();
    if entry.discount_amount > 0.0 {
        parts.push(format!("lesson discount {:.2}", entry.discount_amount));
    }
    if let Some(remark) = entry.remarks.as_ref() {
        let trimmed = remark.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed.to_string());
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

fn nearly_zero(value: f64) -> bool {
    value.abs() < 0.000_001
}

fn clamp_to_i32(value: u32) -> i32 {
    if value > i32::MAX as u32 {
        i32::MAX
    } else {
        value as i32
    }
}

fn run_type_to_db(value: BillingRunType) -> &'static str {
    match value {
        BillingRunType::Preview => "PREVIEW",
        BillingRunType::Final => "FINAL",
    }
}

fn status_to_db(value: BillingRunStatus) -> &'static str {
    match value {
        BillingRunStatus::Pending => "PENDING",
        BillingRunStatus::Running => "RUNNING",
        BillingRunStatus::Failed => "FAILED",
        BillingRunStatus::Completed => "COMPLETED",
    }
}

fn parse_run_type(value: &str) -> BillingRunType {
    match value {
        "FINAL" => BillingRunType::Final,
        _ => BillingRunType::Preview,
    }
}

fn parse_run_status(value: &str) -> BillingRunStatus {
    match value {
        "PENDING" => BillingRunStatus::Pending,
        "RUNNING" => BillingRunStatus::Running,
        "FAILED" => BillingRunStatus::Failed,
        "COMPLETED" => BillingRunStatus::Completed,
        _ => BillingRunStatus::Pending,
    }
}

fn map_db_error(err: sqlx::Error) -> AppError {
    AppError::Database(err.to_string())
}
