use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::RecordId;

/// A concrete unit of work: one file, one destination.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransferJob {
    #[serde(skip_serializing)]
    pub id: Option<RecordId>,
    pub intent: RecordId,
    pub source_path: String,
    pub dest_path: String,
    pub destination: RecordId,
    pub size: i64,
    pub bytes_transferred: i64,
    pub status: JobStatus,
    pub attempts: i64,
    pub max_attempts: i64,
    pub last_error: Option<String>,
    pub error_kind: Option<String>,
    pub source_hash: Option<String>,
    pub dest_hash: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Transferring,
    Verifying,
    Complete,
    Failed,
    NeedsReview,
}
