# Kip API Specification

**Date:** February 21, 2026  
**Status:** Design Document — Authoritative

---

## Overview

This document defines the **public API surface** for Kip's backend engine. Both the GUI (Dioxus) and CLI (clap) consume this API — neither touches the database directly.

### Design Principles

1. **Intent-centric** — All transfers flow through the `Intent` model
2. **Async-first** — All operations return `Result<T, KipError>` futures
3. **Idempotent where possible** — `create_intent` with same params returns existing intent
4. **Progress via callbacks** — Long operations accept `ProgressCallback` for updates
5. **Errors are classified** — `KipError` variants map to specific user actions

---

## Module Structure

```rust
// src/api/mod.rs
pub mod intent;
pub mod transfer;
pub mod location;
pub mod review;
pub mod query;
pub mod config;

pub use intent::*;
pub use transfer::*;
pub use location::*;
pub use review::*;
pub use query::*;
pub use config::*;
```

---

## Core Types

### `KipError`

```rust
#[derive(Debug, Error)]
pub enum KipError {
    // Not found
    #[error("Intent not found: {0}")]
    IntentNotFound(IntentId),
    #[error("Location not found: {0}")]
    LocationNotFound(LocationId),
    #[error("Transfer not found: {0}")]
    TransferNotFound(TransferId),

    // Validation
    #[error("Source path does not exist: {0}")]
    SourcePathNotExists(PathBuf),
    #[error("Source path is not a directory: {0}")]
    SourcePathNotDir(PathBuf),
    #[error("Destination path is not writable: {0}")]
    DestPathNotWritable(PathBuf),
    #[error("Invalid intent configuration: {0}")]
    InvalidIntentConfig(String),

    // Transfer errors
    #[error("Transfer failed: {0}")]
    TransferFailed(TransferError),
    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),
    #[error("Disk full: {0}")]
    DiskFull(PathBuf),
    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    // Database
    #[error("Database error: {0}")]
    Database(String),

    // Config
    #[error("Config import failed: {0}")]
    ConfigImport(String),
}
```

### `TransferError` (classifies review items)

```rust
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
    /// Whether this error should go to review queue vs auto-retry
    pub fn is_retryable(&self) -> bool {
        matches!(self, TransferError::IoError(_) | TransferError::Interrupted)
    }

    pub fn needs_review(&self) -> bool {
        matches!(self, 
            TransferError::SourceNotFound 
            | TransferError::PermissionDenied 
            | TransferError::DiskFull
            | TransferError::HashMismatch
        )
    }
}
```

### Progress Callback

```rust
pub type ProgressCallback = Arc<dyn Fn(ProgressUpdate) + Send + Sync>;

#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub kind: ProgressKind,
    pub current: u64,
    pub total: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ProgressKind {
    Scanning { files_found: u64, bytes_scanned: u64 },
    Transferring { file: String, bytes_transferred: u64 },
    Complete { files_transferred: u64, bytes_transferred: u64 },
}
```

---

## API Functions

### Intent Operations (`api::intent::*`)

#### `create_intent`

```rust
pub async fn create_intent(
    source: LocationId,
    destinations: Vec<LocationId>,
    config: IntentConfig,
) -> Result<IntentId, KipError>;
```

**Behavior:**
- Creates a new intent record in the database
- If an identical intent exists (same source + destinations + config), returns existing ID
- Initial status: `idle`
- Triggers no transfer — caller must call `run_intent()` explicitly

**IntentConfig:**
```rust
#[derive(Debug, Clone)]
pub struct IntentConfig {
    pub name: Option<String>,
    pub speed_mode: SpeedMode,  // "fast" | "throttled" | "background"
    pub priority: u16,           // 0-1000
    pub include_patterns: Vec<String>,  // glob patterns
    pub exclude_patterns: Vec<String>,
    pub bidirectional: bool,
}

#[derive(Debug, Clone)]
pub enum SpeedMode {
    Fast,      // No rate limiting
    Throttled, // Rate-limited (future: bytes/sec limit)
    Background, // Lowest priority, pauses on foreground I/O
}
```

---

#### `delete_intent`

```rust
pub async fn delete_intent(intent: IntentId) -> Result<(), KipError>;
```

**Behavior:**
- Deletes the intent and all associated transfer jobs
- Does NOT delete transferred files
- Idempotent: no error if intent doesn't exist

---

#### `list_intents`

```rust
pub async fn list_intents() -> Result<Vec<IntentSummary>, KipError>;
```

**IntentSummary:**
```rust
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

#[derive(Debug, Clone, PartialEq)]
pub enum IntentStatus {
    Idle,
    Scanning,
    Transferring,
    Complete,
    NeedsReview,
    Error,
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
        if self.total_bytes == 0 { return 0.0; }
        (self.completed_bytes as f64 / self.total_bytes as f64) * 100.0
    }
}
```

---

#### `get_intent`

```rust
pub async fn get_intent(intent: IntentId) -> Result<IntentDetail, KipError>;
```

**IntentDetail:**
```rust
#[derive(Debug, Clone)]
pub struct IntentDetail {
    pub summary: IntentSummary,
    pub config: IntentConfig,
    pub recent_transfers: Vec<TransferSummary>,
}
```

---

### Transfer Operations (`api::transfer::*`)

#### `run_intent`

```rust
pub async fn run_intent(
    intent: IntentId,
    progress: Option<ProgressCallback>,
) -> Result<RunResult, KipError>;
```

**Behavior:**
1. If intent status is `idle`, triggers `scan_intent()` first
2. Creates/updates transfer jobs for all files
3. Runs transfers with bounded concurrency (default: 4 parallel)
4. Reports progress via callback
5. Returns when all jobs are complete, failed, or need review

**RunResult:**
```rust
#[derive(Debug, Clone)]
pub struct RunResult {
    pub completed: u64,
    pub failed: u64,
    pub needs_review: u64,
    pub bytes_transferred: u64,
    pub duration: Duration,
}
```

---

#### `scan_intent`

```rust
pub async fn scan_intent(intent: IntentId) -> Result<ScanResult, KipError>;
```

**Behavior:**
- Walks source filesystem
- Creates transfer jobs for each file → destination pair
- Updates intent with total_files and total_bytes
- Transitions status: `idle` → `scanning` → `transferring` (or `complete` if empty)

**ScanResult:**
```rust
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub files_found: u64,
    pub total_bytes: u64,
    pub jobs_created: u64,
    pub skipped_entries: u64,  // symlinks, permission errors, etc.
}
```

---

#### `cancel_intent`

```rust
pub async fn cancel_intent(intent: IntentId) -> Result<(), KipError>;
```

**Behavior:**
- Sets intent status to `error`
- Marks all `pending` and `transferring` jobs as `cancelled`
- Does not delete already-completed transfers
- Idempotent

---

#### `retry_failed`

```rust
pub async fn retry_failed(intent: IntentId) -> Result<RunResult, KipError>;
```

**Behavior:**
- Resets all `failed` and `needs_review` jobs to `pending`
- Calls `run_intent()` to re-execute
- Returns aggregate result

---

### Location Operations (`api::location::*`)

#### `add_location`

```rust
pub async fn add_location(
    path: PathBuf,
    label: Option<String>,
    machine: Option<MachineId>,
) -> Result<LocationId, KipError>;
```

**Behavior:**
- Validates path exists
- Creates location record
- If machine is None, defaults to `machine:local`
- Returns existing ID if path already registered

---

#### `list_locations`

```rust
pub async fn list_locations() -> Result<Vec<LocationSummary>, KipError>;
```

**LocationSummary:**
```rust
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
    pub id: MachineId,
    pub name: String,
    pub kind: MachineKind,
    pub online: bool,
}

#[derive(Debug, Clone)]
pub enum MachineKind {
    Local,
    Remote { ssh_host: String },
}
```

---

#### `remove_location`

```rust
pub async fn remove_location(location: LocationId) -> Result<(), KipError>;
```

**Behavior:**
- Deletes location record
- Fails if location is referenced by active intents
- Does not delete files

---

### Review Operations (`api::review::*`)

#### `list_review_items`

```rust
pub async fn list_review_items() -> Result<Vec<ReviewItem>, KipError>;
```

**ReviewItem:**
```rust
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
```

---

#### `resolve_review`

```rust
pub async fn resolve_review(
    review: ReviewId,
    resolution: Resolution,
) -> Result<(), KipError>;
```

**Resolution:**
```rust
#[derive(Debug, Clone)]
pub enum Resolution {
    Retry,           // Reset job to pending
    Skip,            // Mark as skipped, continue
    Overwrite,       // Force overwrite (for conflicts)
    DeleteSource,    // Delete source and mark complete
    DeleteDest,      // Delete dest and retry
    AbortIntent,     // Cancel entire intent
}
```

---

#### `resolve_all_review`

```rust
pub async fn resolve_all_review(
    intent: IntentId,
    resolution: Resolution,
) -> Result<u64, KipError>;
```

**Returns:** Count of items resolved

---

### Query Operations (`api::query::*`)

#### `status`

```rust
pub async fn status() -> Result<StatusSummary, KipError>;
```

**StatusSummary:**
```rust
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
    pub by_error: HashMap<TransferError, u64>,
}

#[derive(Debug, Clone)]
pub struct DriveStatus {
    pub connected: Vec<DriveSummary>,
    pub disconnected: Vec<DriveSummary>,
}

#[derive(Debug, Clone)]
pub struct DriveSummary {
    pub id: DriveId,
    pub name: String,
    pub mount_point: Option<String>,
    pub capacity_bytes: Option<u64>,
    pub available: bool,
}
```

---

#### `transfer_history`

```rust
pub async fn transfer_history(
    intent: Option<IntentId>,
    limit: Option<u64>,
) -> Result<Vec<TransferDetail>, KipError>;
```

---

### Config Operations (`api::config::*`)

#### `import_backup_tool_config`

```rust
pub async fn import_backup_tool_config(
    config_dir: Option<PathBuf>,
) -> Result<ImportResult, KipError>;
```

**Behavior:**
- Scans for `folders.toml`, `drives.toml`, `apps/*.toml`
- Creates locations for each source/destination path
- Creates intents matching folder configurations
- Returns summary of what was imported

**ImportResult:**
```rust
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub locations_created: u64,
    pub intents_created: u64,
    pub errors: Vec<ConfigImportError>,
}

#[derive(Debug, Clone)]
pub struct ConfigImportError {
    pub file: PathBuf,
    pub reason: String,
}
```

---

#### `export_config`

```rust
pub async fn export_config(
    format: ConfigFormat,
    output_dir: PathBuf,
) -> Result<(), KipError>;
```

**ConfigFormat:**
```rust
#[derive(Debug, Clone)]
pub enum ConfigFormat {
    Toml,    // backup-tool compatible
    Json,    // Full fidelity
}
```

---

## Error Handling Strategy

### GUI Error Handling

```rust
match api::run_intent(intent_id, None).await {
    Ok(result) => {
        if result.needs_review > 0 {
            show_notification("Transfer complete with errors");
            show_review_panel();
        } else {
            show_notification("Transfer complete");
        }
    }
    Err(KipError::IntentNotFound(id)) => {
        show_error("Intent not found");
    }
    Err(KipError::SourcePathNotExists(path)) => {
        show_error(format!("Source not found: {}", path.display()));
    }
    Err(e) => {
        show_error(format!("Transfer failed: {}", e));
    }
}
```

### CLI Error Handling

```rust
match api::run_intent(intent_id, Some(progress_cb)).await {
    Ok(result) => {
        eprintln!("✅ Complete: {} files, {}", 
            result.completed, 
            format_bytes(result.bytes_transferred)
        );
        if result.needs_review > 0 {
            eprintln!("⚠️  {} items need review", result.needs_review);
            eprintln!("   Run: kip review list");
        }
    }
    Err(e) => {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}
```

---

## Implementation Notes

### What Stays in `engine/`

- `engine/transfer.rs` — copier logic (chunked copy, hash verification)
- `engine/scanner.rs` — filesystem walking, job creation
- `engine/scheduler.rs` — job queue, concurrency control
- `engine/drive.rs` — drive detection via `/Volumes/` polling
- `engine/review.rs` — error classification

### What Moves to `api/`

- All direct `db.query()` calls from GUI
- Intent CRUD operations
- Transfer orchestration (`run_intent` calls `scheduler::run_intent`)
- Location management
- Config import/export

### Database Access Rule

**Only `api::*` modules call `db.query()` directly.**  
Engine modules receive `&DbHandle` from API layer — they don't construct queries themselves.

---

## Future Extensions (Not Yet Implemented)

- `api::schedule::*` — Cron-like scheduling for intents
- `api::remote::*` — SSH tunnel management, remote scanning
- `api::sync::*` — Bidirectional sync conflict resolution
- `api::webhook::*` — External notifications on transfer complete
