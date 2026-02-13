use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::RecordId;

/// A computer or server Ferry knows about.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Machine {
    #[serde(skip_serializing)]
    pub id: Option<RecordId>,
    pub name: String,
    pub kind: MachineKind,
    pub hostname: Option<String>,
    pub is_current: bool,
    pub ssh_user: Option<String>,
    pub ssh_key_path: Option<String>,
    pub ssh_proxy: Option<String>,
    pub last_seen: DateTime<Utc>,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MachineKind {
    Local,
    Remote,
}

/// A removable or mounted storage device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Drive {
    #[serde(skip_serializing)]
    pub id: Option<RecordId>,
    pub name: String,
    pub uuid: String,
    pub filesystem: Option<String>,
    pub capacity_bytes: Option<i64>,
    pub mount_point: Option<String>,
    pub connected: bool,
    pub last_seen: DateTime<Utc>,
    pub limitations: Option<serde_json::Value>,
}

/// A machine/drive + path composition.
/// The fundamental "where" in Ferry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Location {
    #[serde(skip_serializing)]
    pub id: Option<RecordId>,
    pub machine: Option<RecordId>,
    pub drive: Option<RecordId>,
    pub path: String,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub available: bool,
}
