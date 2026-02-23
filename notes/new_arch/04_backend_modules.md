# Backend Engine Module Design

**Date:** February 21, 2026  
**Status:** Design Document

---

## Overview

This document describes the internal structure of Kip's backend engine — the modules that implement the API layer.

### Module Hierarchy

```
src/
├── lib.rs              # Library root, re-exports api::*
├── api/                # Public API surface
│   ├── mod.rs          # Re-exports all public modules
│   ├── intent.rs       # Intent CRUD + run/cancel
│   ├── transfer.rs     # Transfer execution
│   ├── location.rs     # Location management
│   ├── review.rs       # Review queue operations
│   ├── query.rs        # Read-only queries
│   └── config.rs       # Config import/export
├── engine/             # Internal implementation
│   ├── mod.rs
│   ├── transfer.rs     # Low-level copy operations
│   ├── scanner.rs      # Filesystem scanning
│   ├── scheduler.rs    # Job queue + concurrency
│   ├── drive.rs        # Drive detection
│   └── review.rs       # Error classification
├── db/                 # Database layer
│   ├── mod.rs          # Connection + migrations
│   ├── schema.rs       # SCHEMA_V1 constant
│   └── models.rs       # SurrealValue types
└── ui/                 # GUI only (Dioxus)
    └── ...
```

---

## Module: `api::intent`

### Public Functions

```rust
pub async fn create_intent(
    source: LocationId,
    destinations: Vec<LocationId>,
    config: IntentConfig,
) -> Result<IntentId, KipError>

pub async fn delete_intent(intent: IntentId) -> Result<(), KipError>

pub async fn list_intents() -> Result<Vec<IntentSummary>, KipError>

pub async fn get_intent(intent: IntentId) -> Result<IntentDetail, KipError>

pub async fn run_intent(
    intent: IntentId,
    progress: Option<ProgressCallback>,
) -> Result<RunResult, KipError>

pub async fn cancel_intent(intent: IntentId) -> Result<(), KipError>

pub async fn retry_failed(intent: IntentId) -> Result<RunResult, KipError>
```

### Implementation Notes

**`create_intent`:**
```rust
pub async fn create_intent(
    source: LocationId,
    destinations: Vec<LocationId>,
    config: IntentConfig,
) -> Result<IntentId, KipError> {
    // Check for existing identical intent
    let existing = find_existing_intent(&source, &destinations, &config).await?;
    if let Some(id) = existing {
        return Ok(id);
    }

    // Create new intent record
    let intent_id = RecordId::new("intent", generate_ulid());
    db.db
        .query("CREATE $id CONTENT { ... }")
        .bind(("id", intent_id.clone()))
        .bind(("source", source))
        .bind(("destinations", destinations))
        .bind(("status", "idle"))
        .bind(("config", config))
        .await?
        .check()?;

    Ok(intent_id)
}
```

**`run_intent`:**
```rust
pub async fn run_intent(
    intent: IntentId,
    progress: Option<ProgressCallback>,
) -> Result<RunResult, KipError> {
    // 1. Check current status
    let status = get_intent_status(&intent).await?;
    
    // 2. If idle, scan first
    if status == IntentStatus::Idle {
        scan_intent(&intent).await?;
    }

    // 3. Run scheduler
    let result = scheduler::run_intent(&db, &intent, progress).await?;

    // 4. Return result
    Ok(result)
}
```

---

## Module: `api::transfer`

### Public Functions

```rust
pub async fn scan_intent(intent: IntentId) -> Result<ScanResult, KipError>

pub async fn get_transfer_status(transfer: TransferId) -> Result<TransferStatus, KipError>

pub async fn cancel_transfer(transfer: TransferId) -> Result<(), KipError>
```

### Implementation: Delegates to `engine::scanner` and `engine::scheduler`

```rust
pub async fn scan_intent(intent: IntentId) -> Result<ScanResult, KipError> {
    // Validate intent exists
    // Call engine::scanner::scan_intent()
    // Wrap errors
}
```

---

## Module: `api::location`

### Public Functions

```rust
pub async fn add_location(
    path: PathBuf,
    label: Option<String>,
    machine: Option<MachineId>,
) -> Result<LocationId, KipError>

pub async fn list_locations() -> Result<Vec<LocationSummary>, KipError>

pub async fn remove_location(location: LocationId) -> Result<(), KipError>

pub async fn get_location(location: LocationId) -> Result<LocationDetail, KipError>
```

### Implementation Notes

**`add_location`:**
```rust
pub async fn add_location(
    path: PathBuf,
    label: Option<String>,
    machine: Option<MachineId>,
) -> Result<LocationId, KipError> {
    // Expand ~ to home directory
    let path = expand_tilde(path)?;
    
    // Validate path exists
    if !path.exists() {
        return Err(KipError::SourcePathNotExists(path.clone()));
    }

    // Check for existing location with same path
    if let Some(existing) = find_location_by_path(&path).await? {
        return Ok(existing);
    }

    // Resolve machine (default to local)
    let machine = machine.unwrap_or_else(|| RecordId::new("machine", "local"));

    // Create location record
    let location_id = RecordId::new("location", generate_ulid());
    db.db
        .query("CREATE $id CONTENT { path: $path, label: $label, machine: $machine, ... }")
        .bind(("id", location_id))
        .bind(("path", path.to_string_lossy().to_string()))
        .bind(("label", label))
        .bind(("machine", machine))
        .await?
        .check()?;

    Ok(location_id)
}
```

---

## Module: `api::review`

### Public Functions

```rust
pub async fn list_review_items() -> Result<Vec<ReviewItem>, KipError>

pub async fn resolve_review(review: ReviewId, resolution: Resolution) -> Result<(), KipError>

pub async fn resolve_all_review(intent: IntentId, resolution: Resolution) -> Result<u64, KipError>
```

### Implementation Notes

**`resolve_review`:**
```rust
pub async fn resolve_review(review: ReviewId, resolution: Resolution) -> Result<(), KipError> {
    // Load review item
    let item = load_review_item(&review).await?;

    match resolution {
        Resolution::Retry => {
            // Reset job to pending
            db.db
                .query("UPDATE $job SET status = 'pending', error_kind = NONE")
                .bind(("job", item.transfer))
                .await?
                .check()?;
        }
        Resolution::Skip => {
            // Mark job as skipped
            db.db
                .query("UPDATE $job SET status = 'skipped'")
                .bind(("job", item.transfer))
                .await?
                .check()?;
        }
        Resolution::AbortIntent => {
            // Cancel entire intent
            cancel_intent(item.intent).await?;
        }
        // ... other resolutions
    }

    // Mark review item as resolved
    db.db
        .query("UPDATE $id SET resolution = $resolution, resolved_at = time::now()")
        .bind(("id", review))
        .bind(("resolution", format!("{:?}", resolution)))
        .await?
        .check()?;

    Ok(())
}
```

---

## Module: `api::query`

### Public Functions

```rust
pub async fn status() -> Result<StatusSummary, KipError>

pub async fn transfer_history(
    intent: Option<IntentId>,
    limit: Option<u64>,
) -> Result<Vec<TransferDetail>, KipError>

pub async fn get_drive_status() -> Result<DriveStatus, KipError>
```

### Implementation Notes

**`status`:**
```rust
pub async fn status() -> Result<StatusSummary, KipError> {
    // Parallel queries for each section
    let (intent_counts, transfer_counts, review_counts, drive_status) = tokio::try_join!(
        query_intent_counts(),
        query_transfer_counts(),
        query_review_counts(),
        query_drive_status(),
    )?;

    Ok(StatusSummary {
        intents: intent_counts,
        transfers: transfer_counts,
        review_queue: review_counts,
        drives: drive_status,
    })
}
```

---

## Module: `api::config`

### Public Functions

```rust
pub async fn import_backup_tool_config(
    config_dir: Option<PathBuf>,
) -> Result<ImportResult, KipError>

pub async fn export_config(
    format: ConfigFormat,
    output_dir: PathBuf,
) -> Result<(), KipError>
```

### Implementation: Import Logic

```rust
pub async fn import_backup_tool_config(
    config_dir: Option<PathBuf>,
) -> Result<ImportResult, KipError> {
    let config_dir = config_dir.unwrap_or_else(default_backup_tool_config_dir);
    let mut result = ImportResult {
        locations_created: 0,
        intents_created: 0,
        errors: vec![],
    };

    // Load drives.toml
    let drives = match load_drives_config(&config_dir.join("drives.toml")) {
        Ok(d) => d,
        Err(e) => {
            result.errors.push(ConfigImportError {
                file: config_dir.join("drives.toml"),
                reason: e.to_string(),
            });
            vec![]
        }
    };

    // Create drive locations
    for drive in &drives {
        if let Some(mount) = &drive.mount_point {
            let _ = add_location(PathBuf::from(mount), Some(drive.name.clone()), None).await;
            result.locations_created += 1;
        }
    }

    // Load apps/*.toml
    let apps_dir = config_dir.join("apps");
    if apps_dir.exists() {
        for entry in fs::read_dir(&apps_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }

            let app_config = match load_app_config(&path) {
                Ok(c) => c,
                Err(e) => {
                    result.errors.push(ConfigImportError { file: path, reason: e.to_string() });
                    continue;
                }
            };

            // Create source locations and intents
            for folder in &app_config.folder_configs {
                let source_path = expand_tilde(&folder.source)?;
                let source_id = add_location(source_path.clone(), None, None).await?;
                result.locations_created += 1;

                // Create destination locations
                let mut dest_ids = vec![];
                for dest in &folder.destinations {
                    let drive = drives.iter().find(|d| d.name == dest.drive);
                    let dest_path = drive
                        .and_then(|d| d.mount_point.as_ref())
                        .map(|m| PathBuf::from(m).join(&dest.path));
                    
                    if let Some(p) = dest_path {
                        let dest_id = add_location(p, None, None).await?;
                        dest_ids.push(dest_id);
                        result.locations_created += 1;
                    }
                }

                // Create intent
                if !dest_ids.is_empty() {
                    let config = IntentConfig {
                        name: Some(app_config.metadata.name.clone()),
                        priority: app_config.metadata.priority,
                        ..Default::default()
                    };
                    create_intent(source_id, dest_ids, config).await?;
                    result.intents_created += 1;
                }
            }
        }
    }

    Ok(result)
}
```

---

## Module: `engine::transfer` (formerly `copier.rs`)

### Purpose

Low-level file copy operations with hash verification.

### Key Functions

```rust
pub async fn copy_job(
    db: &DbHandle,
    job_id: &RecordId,
) -> Result<CopyResult, CopyError>
```

### Implementation

- Chunked copying (256KB chunks)
- Progress updates to DB every 4 chunks (~1MB)
- BLAKE3 hash verification
- Status transitions: `pending` → `transferring` → `complete` / `needs_review`

---

## Module: `engine::scanner`

### Purpose

Filesystem scanning and transfer job creation.

### Key Functions

```rust
pub async fn scan_intent(
    db: &DbHandle,
    intent_id: &RecordId,
) -> Result<ScanResult, ScanError>
```

### Implementation

- Uses `walkdir` for recursive traversal
- Skips symlinks
- Creates `transfer_job` records for each file → destination pair
- Handles include/exclude patterns (future)

---

## Module: `engine::scheduler`

### Purpose

Job queue management with bounded concurrency.

### Key Functions

```rust
pub async fn run_intent(
    db: &DbHandle,
    intent_id: &RecordId,
    progress: Option<ProgressCallback>,
) -> Result<RunResult, SchedulerError>
```

### Implementation

- Recovery: resets `transferring` jobs from crashed runs
- Semaphore-based concurrency (max 4 parallel)
- Loops until no pending jobs remain
- Aggregates results into `RunResult`

---

## Module: `engine::drive`

### Purpose

Drive detection via `/Volumes/` polling on macOS.

### Key Functions

```rust
pub struct DriveWatcher;

impl DriveWatcher {
    pub fn start(db: DbHandle) -> Self;
    pub fn get_connected_drives() -> Vec<DriveInfo>;
}
```

### Implementation

- Polls `/Volumes/` every 5 seconds
- Uses `diskutil info -plist` for volume metadata
- Updates `drive` table with `connected` status

---

## Module: `db`

### Purpose

Database connection, migrations, and models.

### Structure

```rust
// db/mod.rs
pub use self::handle::DbHandle;
pub use self::schema::SCHEMA_V1;
pub use self::models::*;

pub async fn init() -> Result<DbHandle, Box<dyn Error>>;
```

### Schema Location

```rust
// db/schema.rs
pub const SCHEMA_V1: &str = r#"
    DEFINE TABLE OVERWRITE machine SCHEMAFULL;
    DEFINE FIELD OVERWRITE name ON machine TYPE string;
    -- ... full schema
"#;
```

---

## Error Handling Strategy

### Error Flow

```
engine::transfer::CopyError
         ↓
api::transfer wraps to KipError::TransferFailed
         ↓
CLI/GUI handles KipError variants
```

### Error Classification

```rust
impl From<CopyError> for KipError {
    fn from(err: CopyError) -> Self {
        match err {
            CopyError::SourceNotFound(_) => KipError::TransferFailed(TransferError::SourceNotFound),
            CopyError::PermissionDenied(_) => KipError::TransferFailed(TransferError::PermissionDenied),
            CopyError::HashMismatch { .. } => KipError::TransferFailed(TransferError::HashMismatch),
            CopyError::IoError(msg) => KipError::TransferFailed(TransferError::IoError(msg)),
            CopyError::DbError(msg) => KipError::Database(msg),
            _ => KipError::TransferFailed(TransferError::IoError(err.to_string())),
        }
    }
}
```

---

## Testing Strategy

### Unit Tests (engine modules)

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_walk_source_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let (entries, skipped) = walk_source(tmp.path()).unwrap();
        assert_eq!(entries.len(), 0);
        assert_eq!(skipped, 0);
    }
}
```

### Integration Tests (API layer)

```rust
#[tokio::test]
async fn test_create_and_run_intent() {
    let db = setup_test_db().await;
    
    let source = api::add_location(test_path(), None, None).await.unwrap();
    let dest = api::add_location(test_dest(), None, None).await.unwrap();
    
    let intent = api::create_intent(source, vec![dest], IntentConfig::default()).await.unwrap();
    let result = api::run_intent(intent, None).await.unwrap();
    
    assert!(result.completed > 0);
}
```
