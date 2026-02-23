# Migration Plan: Current State → Unified Architecture

**Date:** February 21, 2026  
**Status:** Design Document

---

## Overview

This document describes the step-by-step migration from the current state to the unified architecture.

**Important:** The existing codebase has gaps between documentation and reality. See `06_codebase_insights.md` for accurate status.

### Phases

```
Phase 1: Foundation      — CLI skeleton, API module structure
Phase 2: Backend Extract — Move engine code into library  
Phase 3: Unify Database  — Single db::init(), shared schema
Phase 4: Config Import   — Import backup-tool TOML configs
Phase 5: GUI Migration   — Update GUI to use API layer
Phase 6: Wire Engine     — Connect UI actions to engine
Phase 7: Polish          — Error handling, tests, docs
```

---

## Phase 1: Foundation

**Goal:** Create CLI binary skeleton and API module structure.

### Step 1.1: Update Cargo.toml

```toml
# Add CLI binary
[[bin]]
name = "kip-cli"
path = "src/bin/kip-cli.rs"

[dependencies]
clap = { version = "4.4", features = ["derive"] }
# ... existing deps
```

### Step 1.2: Create Module Structure

```bash
mkdir -p src/api src/bin
touch src/api/mod.rs src/api/intent.rs src/api/transfer.rs
touch src/api/location.rs src/api/review.rs src/api/query.rs src/api/config.rs
touch src/bin/kip-cli.rs
```

### Step 1.3: Create CLI Skeleton

```rust
// src/bin/kip-cli.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kip")]
#[command(about = "File transfer orchestrator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show system status
    Status,
    /// Manage intents
    Intent {
        #[command(subcommand)]
        command: IntentCommands,
    },
    // ... more commands
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        match cli.command {
            Commands::Status => cmd_status().await,
            Commands::Intent { command } => cmd_intent(command).await,
        }
    })
}

async fn cmd_status() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement
    println!("Status not yet implemented");
    Ok(())
}
```

### Step 1.4: Create API Module Skeletons

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

// Re-export common types
pub use crate::db::DbHandle;
```

### Step 1.5: Verify Build

```bash
dx build
# Should compile with stubs
```

---

## Phase 2: Backend Extract

**Goal:** Move engine code into library, create clean module structure.

### Step 2.1: Reorganize Engine Modules

Current:
```
src/engine/
├── copier.rs
├── scanner.rs
├── scheduler.rs
└── mod.rs
```

Target:
```
src/engine/
├── mod.rs          # Re-exports
├── transfer.rs     # copier.rs → transfer.rs
├── scanner.rs      # (keep)
├── scheduler.rs    # (keep)
├── drive.rs        # (new, from devices/)
└── review.rs       # (new, error classification)
```

### Step 2.2: Rename copier.rs → transfer.rs

```bash
mv src/engine/copier.rs src/engine/transfer.rs
```

Update imports:
```rust
// In scheduler.rs
use crate::engine::transfer::copy_job;  // was copier::copy_job
```

### Step 2.3: Extract Drive Detection

Move drive detection from `devices/` to `engine/drive.rs`:

```rust
// src/engine/drive.rs
use crate::db::DbHandle;

pub struct DriveWatcher {
    // ...
}

impl DriveWatcher {
    pub fn start(db: DbHandle) -> Self {
        // Existing logic from devices/drive_watcher.rs
    }
    
    pub fn get_connected_drives() -> Vec<DriveInfo> {
        // Existing logic
    }
}
```

### Step 2.4: Update lib.rs

```rust
// src/lib.rs
pub mod api;
pub mod engine;
pub mod db;
pub mod devices;  // Keep for GUI-specific device handling
pub mod util;

// Re-export API for CLI consumption
pub use api::*;
```

---

## Phase 3: Unify Database

**Goal:** Single `db::init()` shared by both binaries.

### Step 3.1: Create db/mod.rs

```rust
// src/db/mod.rs
mod handle;
mod schema;
mod models;

pub use handle::DbHandle;
pub use schema::SCHEMA_V1;
pub use models::*;

pub async fn init() -> Result<DbHandle, Box<dyn std::error::Error>> {
    let path = db_path();
    let db = Surreal::new::<SurrealKv>(path).await?;
    db.use_ns("kip").use_db("kip").await?;
    
    run_migrations(&db).await?;
    bootstrap_local_machine(&db).await?;
    
    Ok(DbHandle { db })
}
```

### Step 3.2: Extract Schema

```rust
// src/db/schema.rs
pub const SCHEMA_V1: &str = r#"
    DEFINE TABLE OVERWRITE machine SCHEMAFULL;
    DEFINE FIELD OVERWRITE name ON machine TYPE string;
    -- ... full schema from kip/src/db.rs
"#;
```

### Step 3.3: Update GUI main.rs

```rust
// src/main.rs (GUI)
use kip::{db, app};

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let db_result = rt.block_on(async { db::init().await });
    Box::leak(Box::new(rt));
    
    match db_result {
        Ok(db) => LaunchBuilder::new().with_context(db).launch(app::App),
        Err(e) => { /* error handling */ }
    }
}
```

### Step 3.4: Update CLI main.rs

```rust
// src/bin/kip-cli.rs
use kip::db;

async fn cmd_status() -> Result<(), Box<dyn std::error::Error>> {
    let db = db::init().await?;
    // Use db for queries
}
```

---

## Phase 4: Config Import

**Goal:** Import backup-tool TOML configs into SurrealDB.

### Step 4.1: Copy Config Loading Code

From `backup-tool/src/config.rs`:
```rust
// src/api/config.rs
use std::path::PathBuf;
use serde::Deserialize;

#[derive(Deserialize)]
struct BackupConfig {
    drives: Vec<DriveConfig>,
    folder_configs: Vec<FolderConfig>,
    // ...
}

pub async fn import_backup_tool_config(
    config_dir: Option<PathBuf>,
) -> Result<ImportResult, KipError> {
    // Implementation from design doc
}
```

### Step 4.2: Add Import Command to CLI

```rust
// src/bin/kip-cli.rs
#[derive(Subcommand)]
enum Commands {
    // ...
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Import backup-tool configuration
    Import {
        #[arg(long)]
        config_dir: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
}

async fn cmd_config_import(config_dir: Option<PathBuf>, dry_run: bool) -> Result<()> {
    let result = kip::api::import_backup_tool_config(config_dir).await?;
    
    if dry_run {
        println!("Would import:");
        println!("  Locations: {}", result.locations_created);
        println!("  Intents: {}", result.intents_created);
    } else {
        println!("Import complete:");
        println!("  Locations: {}", result.locations_created);
        println!("  Intents: {}", result.intents_created);
    }
    
    Ok(())
}
```

### Step 4.3: Test Import

```bash
# From backup-tool config directory
kip config import --dry-run
kip config import
kip intent list  # Verify imported intents
```

---

## Phase 5: GUI Migration

**Goal:** Update GUI to use API layer instead of direct DB calls.

### Step 5.1: Identify Direct DB Calls in GUI

Current GUI code calls `db.db.query()` directly in:
- `src/ui/graph_store.rs` — `create_edge_in_db()`, `trigger_transfer()`
- `src/ui/graph.rs` — Hostname query, drive watcher
- `src/app.rs` — Hostname query

### Step 5.2: Create API Wrappers

```rust
// src/api/intent.rs (add)
pub async fn create_intent_from_locations(
    source_id: &str,
    dest_id: &str,
) -> Result<IntentId, KipError> {
    // Wrap existing graph_store logic
}
```

### Step 5.3: Update graph_store.rs

```rust
// Before
pub async fn create_edge_in_db(
    db: &DbHandle,
    source: &GraphNode,
    dest: &GraphNode,
) -> Result<String, anyhow::Error> {
    db.db.query("CREATE intent:...").await?...;
}

// After
pub async fn create_edge_in_db(
    db: &DbHandle,
    source: &GraphNode,
    dest: &GraphNode,
) -> Result<String, anyhow::Error> {
    let source_loc = resolve_location(db, source).await?;
    let dest_loc = resolve_location(db, dest).await?;
    
    let intent_id = kip::api::create_intent(source_loc, vec![dest_loc], IntentConfig::default()).await?;
    Ok(intent_id.to_string())
}
```

### Step 5.4: Update trigger_transfer

```rust
// Before
tokio::spawn(async move {
    scheduler::run_intent(&db, &intent_id).await?;
});

// After
tokio::spawn(async move {
    kip::api::run_intent(intent_id, None).await?;
});
```

---

## Phase 6: Wire Engine

**Goal:** Connect UI actions to backend engine execution.

### Step 6.1: Fix Directory Expansion

**Location:** `src/ui/graph.rs` — expansion handler

**Problem:** Only Machine/Drive nodes trigger `scan_directory()`.

**Fix:**
```rust
match &node_kind {
    NodeKind::Machine { .. } | NodeKind::Drive { .. } => {
        scan_directory(&path, ...);
    }
    NodeKind::Directory { .. } => {
        // Add: Also scan directory filesystem
        scan_directory(&path, ...);
    }
    _ => {}
}
```

### Step 6.2: Verify Edge Drop Handler

**Location:** `src/ui/graph.rs` — `onmouseup` handler

**Test:**
1. Ctrl+click node A
2. Drag to node B
3. Release on B
4. Verify edge appears and persists

**Fix if needed:**
```rust
// In node mousedown handler during CreatingEdge state:
if let DragState::CreatingEdge { source_id } = &drag_state {
    create_edge_in_db(&db, source_id, &target_id).await;
    graph.with_mut(|g| g.add_edge(new_edge));
}
```

### Step 6.3: Fix Multi-Drag

**Location:** `src/ui/graph.rs` — `DragState::Dragging` handler

**Problem:** Lasso selects multiple, but drag moves only one.

**Fix:**
```rust
// Track all selected nodes
let selected_ids: Vec<String> = graph().selected.iter().cloned().collect();

// Apply same offset to all selected
for id in &selected_ids {
    if let Some(node) = graph.find_node_mut(id) {
        node.position.x += delta_x;
        node.position.y += delta_y;
    }
}
```

### Step 6.4: Wire Run Intent

**Location:** Review queue, intent actions

**Add:** Button to run intent that calls `api::run_intent()`

---

## Phase 7: Polish

**Goal:** Error handling, tests, documentation.

### Step 7.1: Implement Missing API Functions

Checklist:
- [ ] `api::intent::delete_intent()`
- [ ] `api::intent::cancel_intent()`
- [ ] `api::location::remove_location()`
- [ ] `api::review::resolve_review()`
- [ ] `api::query::transfer_history()`

### Step 7.2: Add Unit Tests

```rust
// tests/api_intent.rs
#[tokio::test]
async fn test_create_intent_idempotent() {
    let db = setup_test_db().await;
    let source = create_test_location(&db, "/source").await;
    let dest = create_test_location(&db, "/dest").await;
    
    let intent1 = api::create_intent(source.clone(), vec![dest.clone()], IntentConfig::default()).await.unwrap();
    let intent2 = api::create_intent(source, vec![dest], IntentConfig::default()).await.unwrap();
    
    assert_eq!(intent1, intent2);
}
```

### Step 7.3: Add Integration Tests

```bash
# Test full workflow
kip config import --config-dir ~/.config/backup-tool
kip intent list
kip intent run intent:backup_obsidian
kip status
```

### Step 7.4: Update Documentation

- [ ] Update `AGENTS.md` with new module structure
- [ ] Add README.md for CLI commands
- [ ] Update `the_design/` docs with current status

---

## File Movement Summary

| From | To | Notes |
|------|-----|-------|
| `backup-tool/src/config.rs` | `kip/src/api/config.rs` | Adapt for API |
| `backup-tool/src/drive_config.rs` | `kip/src/engine/drive.rs` | Merge with drive detection |
| `kip/src/engine/copier.rs` | `kip/src/engine/transfer.rs` | Rename |
| `kip/src/db.rs` | `kip/src/db/mod.rs` | Split into modules |
| `kip/src/devices/drive_watcher.rs` | `kip/src/engine/drive.rs` | Merge |

---

## Rollback Plan

If migration fails:

1. **Preserve backup-tool** — Keep original directory intact
2. **Git branch per phase** — `migration-phase-1`, `migration-phase-2`, etc.
3. **Test import separately** — Run import on copy of config, verify before committing
4. **Dual-write during transition** — Write to both old and new schemas temporarily

---

## Verification Checklist

After each phase:

```bash
# Build
dx build

# CLI basic commands
kip status
kip intent list
kip location list

# GUI launch
dx serve --platform desktop

# Import test (Phase 4+)
kip config import --dry-run
```

---

## Timeline Estimate

| Phase | Estimated Time |
|-------|----------------|
| Phase 1: Foundation | 2-3 hours |
| Phase 2: Backend Extract | 3-4 hours |
| Phase 3: Unify Database | 2-3 hours |
| Phase 4: Config Import | 4-6 hours |
| Phase 5: GUI Migration | 4-6 hours |
| Phase 6: Wire Engine | 4-6 hours |
| Phase 7: Polish | 3-4 hours |
| **Total** | **22-32 hours** |
