use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::RecordId;

/// The core transfer declaration.
/// "I want files from here to end up there."
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Intent {
    #[serde(skip_serializing)]
    pub id: Option<RecordId>,
    pub name: Option<String>,
    pub source: RecordId,
    pub destinations: Vec<RecordId>,
    pub status: IntentStatus,
    pub kind: IntentKind,
    pub speed_mode: SpeedMode,
    pub priority: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_files: i64,
    pub total_bytes: i64,
    pub completed_files: i64,
    pub completed_bytes: i64,
    pub include_patterns: Option<Vec<String>>,
    pub exclude_patterns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    Idle,
    Scanning,
    Transferring,
    Verifying,
    Complete,
    WaitingForDevice,
    NeedsReview,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IntentKind {
    OneShot,
    Sync,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SpeedMode {
    Normal,
    Ninja,
    Blast,
}

impl Default for SpeedMode {
    fn default() -> Self {
        SpeedMode::Normal
    }
}
