use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 班级状态，映射 `classes.status`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClassStatus {
    Planned,
    Active,
    Archived,
}

impl ClassStatus {
    pub fn from_str(value: &str) -> Self {
        match value {
            "ACTIVE" => ClassStatus::Active,
            "ARCHIVED" => ClassStatus::Archived,
            _ => ClassStatus::Planned,
        }
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            ClassStatus::Planned => "PLANNED",
            ClassStatus::Active => "ACTIVE",
            ClassStatus::Archived => "ARCHIVED",
        }
    }
}

/// `classes` 表对应的领域模型。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInstance {
    pub id: Uuid,
    pub term_id: Uuid,
    pub campus_id: Uuid,
    pub club_id: Uuid,
    pub class_code: String,
    pub weekday: u8,
    #[serde(with = "time_format")]
    pub start_time: NaiveTime,
    #[serde(with = "time_format")]
    pub end_time: NaiveTime,
    pub location: Option<String>,
    pub capacity: Option<i32>,
    pub status: ClassStatus,
    pub notes: Option<String>,
}

mod time_format {
    use chrono::NaiveTime;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const DISPLAY_FMT: &str = "%H:%M";
    const FALLBACK_FMT: &str = "%H:%M:%S";

    pub fn serialize<S>(value: &NaiveTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.format(DISPLAY_FMT).to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        NaiveTime::parse_from_str(&raw, DISPLAY_FMT)
            .or_else(|_| NaiveTime::parse_from_str(&raw, FALLBACK_FMT))
            .map_err(serde::de::Error::custom)
    }
}
