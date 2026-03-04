use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::RecordId;

/// An error that needs human attention.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewItem {
	#[serde(skip_serializing)]
	pub id: Option<RecordId>,
	pub job: RecordId,
	pub intent: RecordId,
	pub error_kind: ErrorKind,
	pub error_message: String,
	pub source_path: String,
	pub dest_path: String,
	pub options: Vec<String>,
	pub resolution: Option<String>,
	pub created_at: DateTime<Utc>,
	pub resolved_at: Option<DateTime<Utc>>,
	pub source_size: Option<i64>,
	pub source_hash: Option<String>,
	pub source_modified: Option<DateTime<Utc>>,
	pub dest_size: Option<i64>,
	pub dest_hash: Option<String>,
	pub dest_modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
	Conflict,
	PermissionDenied,
	DiskFull,
	FileTooLarge,
	NameInvalid,
	SourceMissing,
	HashMismatch,
	AuthFailed,
}
