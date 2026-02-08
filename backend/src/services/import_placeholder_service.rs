use std::{collections::HashSet, fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{db::DbPool, error::AppError};

const DEFAULT_ENROLLMENT_PLACEHOLDERS: &[&str] = &[
    "-",
    "—",
    "——",
    "无",
    "N/A",
    "n/a",
    "NA",
    "na",
    "(空)",
    "（空）",
    "(跳过)",
];
const DEFAULT_STUDENT_PLACEHOLDERS: &[&str] = &[];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ImportPlaceholderType {
    Enrollments,
    Students,
}

impl ImportPlaceholderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Enrollments => "ENROLLMENTS",
            Self::Students => "STUDENTS",
        }
    }

    fn defaults(&self) -> &'static [&'static str] {
        match self {
            Self::Enrollments => DEFAULT_ENROLLMENT_PLACEHOLDERS,
            Self::Students => DEFAULT_STUDENT_PLACEHOLDERS,
        }
    }

    pub fn all() -> &'static [ImportPlaceholderType] {
        const ALL: [ImportPlaceholderType; 2] = [
            ImportPlaceholderType::Enrollments,
            ImportPlaceholderType::Students,
        ];
        &ALL
    }
}

impl fmt::Display for ImportPlaceholderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ImportPlaceholderType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "ENROLLMENTS" => Ok(Self::Enrollments),
            "STUDENTS" => Ok(Self::Students),
            other => Err(AppError::Validation(format!(
                "不支持的 import_type: {}",
                other
            ))),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ImportPlaceholderConfig {
    pub import_type: ImportPlaceholderType,
    pub placeholders: Vec<String>,
    pub updated_by: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct ImportPlaceholderService<'a> {
    pool: &'a DbPool,
}

impl<'a> ImportPlaceholderService<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn list_all(&self) -> Result<Vec<ImportPlaceholderConfig>, AppError> {
        let mut configs = Vec::new();
        for import_type in ImportPlaceholderType::all() {
            configs.push(self.get_or_default(*import_type).await?);
        }
        Ok(configs)
    }

    pub async fn get_or_default(
        &self,
        import_type: ImportPlaceholderType,
    ) -> Result<ImportPlaceholderConfig, AppError> {
        if let Some(config) = self.fetch(import_type).await? {
            Ok(config)
        } else {
            Ok(ImportPlaceholderConfig {
                import_type,
                placeholders: import_type
                    .defaults()
                    .iter()
                    .map(|value| value.to_string())
                    .collect(),
                updated_by: None,
                updated_at: Utc::now(),
            })
        }
    }

    pub async fn resolved_values(
        &self,
        import_type: ImportPlaceholderType,
    ) -> Result<Vec<String>, AppError> {
        let config = self.get_or_default(import_type).await?;
        Ok(config.placeholders)
    }

    pub async fn replace(
        &self,
        import_type: ImportPlaceholderType,
        placeholders: Vec<String>,
        updated_by: &str,
    ) -> Result<ImportPlaceholderConfig, AppError> {
        let cleaned = Self::clean_inputs(placeholders);
        let row = sqlx::query(
            r#"
                INSERT INTO import_placeholder_sets (import_type, placeholders, updated_by)
                VALUES ($1,$2,$3)
                ON CONFLICT (import_type)
                DO UPDATE SET
                    placeholders = EXCLUDED.placeholders,
                    updated_by = EXCLUDED.updated_by,
                    updated_at = now()
                RETURNING import_type, placeholders, updated_by, updated_at
            "#,
        )
        .bind(import_type.as_str())
        .bind(&cleaned)
        .bind(updated_by)
        .fetch_one(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        Self::map_row(row)
    }

    async fn fetch(
        &self,
        import_type: ImportPlaceholderType,
    ) -> Result<Option<ImportPlaceholderConfig>, AppError> {
        let row = sqlx::query(
            r#"
                SELECT import_type, placeholders, updated_by, updated_at
                FROM import_placeholder_sets
                WHERE import_type = $1
            "#,
        )
        .bind(import_type.as_str())
        .fetch_optional(self.pool)
        .await
        .map_err(|err| AppError::Database(err.to_string()))?;

        row.map(Self::map_row).transpose()
    }

    fn map_row(row: sqlx::postgres::PgRow) -> Result<ImportPlaceholderConfig, AppError> {
        let raw_type: String = row
            .try_get("import_type")
            .map_err(|err| AppError::Database(err.to_string()))?;
        let import_type = ImportPlaceholderType::from_str(&raw_type)?;
        let placeholders: Vec<String> = row
            .try_get("placeholders")
            .map_err(|err| AppError::Database(err.to_string()))?;
        let updated_by: Option<String> = row
            .try_get("updated_by")
            .map_err(|err| AppError::Database(err.to_string()))?;
        let updated_at: DateTime<Utc> = row
            .try_get("updated_at")
            .map_err(|err| AppError::Database(err.to_string()))?;

        Ok(ImportPlaceholderConfig {
            import_type,
            placeholders,
            updated_by,
            updated_at,
        })
    }

    fn clean_inputs(values: Vec<String>) -> Vec<String> {
        if values.is_empty() {
            return Vec::new();
        }

        let mut seen = HashSet::new();
        let mut cleaned = Vec::new();
        for raw in values {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let normalized = trimmed.to_lowercase();
            if seen.insert(normalized) {
                cleaned.push(trimmed.to_string());
            }
        }
        cleaned
    }
}
