use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::RecordId;

/// Every file Kip has ever touched.
/// The basis for dedup and change detection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileRecord {
    #[serde(skip_serializing)]
    pub id: Option<RecordId>,
    pub hash: String,
    pub size: i64,
    pub first_seen: DateTime<Utc>,
}

/// Graph edge: file_record -> exists_at -> location.
/// Created via RELATE, not direct insert.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExistsAt {
    #[serde(skip_serializing)]
    pub id: Option<RecordId>,
    #[serde(rename = "in")]
    pub from: Option<RecordId>,
    #[serde(rename = "out")]
    pub to: Option<RecordId>,
    pub path: String,
    pub modified_at: DateTime<Utc>,
    pub verified_at: DateTime<Utc>,
    pub stale: bool,
}
