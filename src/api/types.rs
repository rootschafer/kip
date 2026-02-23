//! Common types used across the Kip API

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unique identifier for intents
pub type IntentId = String;

/// Unique identifier for locations
pub type LocationId = String;

/// Unique identifier for transfers
pub type TransferId = String;

/// Unique identifier for review items
pub type ReviewId = String;

/// Progress callback type for long-running operations
pub type ProgressCallback = Arc<dyn Fn(ProgressUpdate) + Send + Sync>;

/// Transfer error classification - determines if retry or review is needed
#[derive(Debug, Clone)]
pub enum TransferError {
	SourceNotFound,
	PermissionDenied,
	DiskFull,
	HashMismatch,
	IoError(String),
	Interrupted,
}

impl TransferError {
	/// Whether this error should be auto-retried vs. go to review queue
	pub fn is_retryable(&self) -> bool {
		matches!(self, TransferError::IoError(_) | TransferError::Interrupted)
	}

	pub fn needs_review(&self) -> bool {
		matches!(
			self,
			TransferError::SourceNotFound
				| TransferError::PermissionDenied
				| TransferError::DiskFull
				| TransferError::HashMismatch
		)
	}
}

impl std::fmt::Display for TransferError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			TransferError::SourceNotFound => write!(f, "Source not found"),
			TransferError::PermissionDenied => write!(f, "Permission denied"),
			TransferError::DiskFull => write!(f, "Disk full"),
			TransferError::HashMismatch => write!(f, "Hash mismatch"),
			TransferError::IoError(msg) => write!(f, "I/O error: {}", msg),
			TransferError::Interrupted => write!(f, "Transfer interrupted"),
		}
	}
}

/// Main Kip error type
#[derive(Debug, Error)]
pub enum KipError {
	// Not found
	#[error("Intent not found: {0}")]
	IntentNotFound(String),
	#[error("Location not found: {0}")]
	LocationNotFound(String),
	#[error("Transfer not found: {0}")]
	TransferNotFound(String),
	#[error("Review item not found: {0}")]
	ReviewNotFound(String),

	// Validation
	#[error("Source path does not exist: {0}")]
	SourcePathNotExists(std::path::PathBuf),
	#[error("Source path is not a directory: {0}")]
	SourcePathNotDir(std::path::PathBuf),
	#[error("Destination path is not writable: {0}")]
	DestPathNotWritable(std::path::PathBuf),
	#[error("Invalid intent configuration: {0}")]
	InvalidIntentConfig(String),

	// Transfer errors
	#[error("Transfer failed: {0}")]
	TransferFailed(TransferError),
	#[error("Permission denied: {0}")]
	PermissionDenied(std::path::PathBuf),
	#[error("Disk full: {0}")]
	DiskFull(std::path::PathBuf),
	#[error("Hash mismatch: expected {expected}, got {actual}")]
	HashMismatch { expected: String, actual: String },

	// Database
	#[error("Database error: {0}")]
	Database(String),

	// Config
	#[error("Config import failed: {0}")]
	ConfigImport(String),

	// IO
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),
}

impl From<surrealdb::Error> for KipError {
	fn from(err: surrealdb::Error) -> Self {
		KipError::Database(err.to_string())
	}
}

#[derive(Debug, Clone)]
pub struct ProgressUpdate {
	pub kind: ProgressKind,
	pub current: u64,
	pub total: u64,
	pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ProgressKind {
	Scanning {
		files_found: u64,
		bytes_scanned: u64,
	},
	Transferring {
		file: String,
		bytes_transferred: u64,
	},
	Complete {
		files_transferred: u64,
		bytes_transferred: u64,
	},
}

/// Intent status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IntentStatus {
	Idle,
	Scanning,
	Transferring,
	Complete,
	NeedsReview,
	Error,
}

impl std::fmt::Display for IntentStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			IntentStatus::Idle => write!(f, "idle"),
			IntentStatus::Scanning => write!(f, "scanning"),
			IntentStatus::Transferring => write!(f, "transferring"),
			IntentStatus::Complete => write!(f, "complete"),
			IntentStatus::NeedsReview => write!(f, "needs_review"),
			IntentStatus::Error => write!(f, "error"),
		}
	}
}

/// Intent kind
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IntentKind {
	Backup,
	Sync,
	Archive,
}

/// Speed mode for transfers
#[derive(Debug, Clone, Default)]
pub enum SpeedMode {
	#[default]
	Fast,
	Throttled,
	Background,
}

/// Configuration for creating an intent
#[derive(Debug, Clone, Default)]
pub struct IntentConfig {
	pub name: Option<String>,
	pub speed_mode: SpeedMode,
	pub priority: u16,
	pub include_patterns: Vec<String>,
	pub exclude_patterns: Vec<String>,
	pub bidirectional: bool,
}

/// Summary of an intent
#[derive(Debug, Clone)]
pub struct IntentSummary {
	pub id: IntentId,
	pub name: Option<String>,
	pub source: LocationSummary,
	pub destinations: Vec<LocationSummary>,
	pub status: IntentStatus,
	pub kind: IntentKind,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub progress: IntentProgress,
}

#[derive(Debug, Clone)]
pub struct IntentProgress {
	pub total_files: u64,
	pub total_bytes: u64,
	pub completed_files: u64,
	pub completed_bytes: u64,
}

impl IntentProgress {
	pub fn percent_complete(&self) -> f64 {
		if self.total_bytes == 0 {
			return 0.0;
		}
		(self.completed_bytes as f64 / self.total_bytes as f64) * 100.0
	}
}

/// Detailed intent information
#[derive(Debug, Clone)]
pub struct IntentDetail {
	pub summary: IntentSummary,
	pub config: IntentConfig,
	pub recent_transfers: Vec<TransferSummary>,
}

/// Location summary
#[derive(Debug, Clone)]
pub struct LocationSummary {
	pub id: LocationId,
	pub path: String,
	pub label: Option<String>,
	pub machine: MachineSummary,
	pub available: bool,
}

#[derive(Debug, Clone)]
pub struct MachineSummary {
	pub id: String,
	pub name: String,
	pub kind: MachineKind,
	pub online: bool,
}

#[derive(Debug, Clone)]
pub enum MachineKind {
	Local,
	Remote { ssh_host: String },
}

/// Transfer summary
#[derive(Debug, Clone)]
pub struct TransferSummary {
	pub id: TransferId,
	pub source_path: String,
	pub dest_path: String,
	pub status: String,
	pub size: u64,
	pub bytes_transferred: u64,
	pub completed_at: Option<DateTime<Utc>>,
}

/// Scan result
#[derive(Debug, Clone)]
pub struct ScanResult {
	pub files_found: u64,
	pub total_bytes: u64,
	pub jobs_created: u64,
	pub skipped_entries: u64,
}

/// Run result for an intent
#[derive(Debug, Clone)]
pub struct RunResult {
	pub completed: u64,
	pub failed: u64,
	pub needs_review: u64,
	pub bytes_transferred: u64,
	pub duration: std::time::Duration,
}

/// Review item
#[derive(Debug, Clone)]
pub struct ReviewItem {
	pub id: ReviewId,
	pub intent: IntentId,
	pub transfer: TransferId,
	pub error_kind: TransferError,
	pub error_message: String,
	pub source_path: String,
	pub dest_path: String,
	pub source_info: Option<FileMetadata>,
	pub dest_info: Option<FileMetadata>,
	pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
	pub size: u64,
	pub hash: String,
	pub modified: DateTime<Utc>,
}

/// Resolution for a review item
#[derive(Debug, Clone)]
pub enum Resolution {
	Retry,
	Skip,
	Overwrite,
	DeleteSource,
	DeleteDest,
	AbortIntent,
}

/// Status summary for the system
#[derive(Debug, Clone)]
pub struct StatusSummary {
	pub intents: IntentCounts,
	pub transfers: TransferCounts,
	pub review_queue: ReviewCounts,
	pub drives: DriveStatus,
}

#[derive(Debug, Clone)]
pub struct IntentCounts {
	pub total: u64,
	pub idle: u64,
	pub transferring: u64,
	pub complete: u64,
	pub needs_review: u64,
}

#[derive(Debug, Clone)]
pub struct TransferCounts {
	pub pending: u64,
	pub transferring: u64,
	pub complete: u64,
	pub failed: u64,
	pub needs_review: u64,
}

#[derive(Debug, Clone)]
pub struct ReviewCounts {
	pub total: u64,
	pub by_error: std::collections::HashMap<String, u64>,
}

#[derive(Debug, Clone)]
pub struct DriveStatus {
	pub connected: Vec<DriveSummary>,
	pub disconnected: Vec<DriveSummary>,
}

#[derive(Debug, Clone)]
pub struct DriveSummary {
	pub id: String,
	pub name: String,
	pub mount_point: Option<String>,
	pub capacity_bytes: Option<u64>,
	pub available: bool,
}

/// Result of config import
#[derive(Debug, Clone)]
pub struct ImportResult {
	pub locations_created: u64,
	pub intents_created: u64,
	pub errors: Vec<ConfigImportError>,
}

#[derive(Debug, Clone)]
pub struct ConfigImportError {
	pub file: std::path::PathBuf,
	pub reason: String,
}

/// Config export format
#[derive(Debug, Clone)]
pub enum ConfigFormat {
	Toml,
	Json,
}
