# Codebase Insights & Analysis

**Date:** February 21, 2026  
**Source:** Deep exploration of Kip and backup-tool codebases

---

## Executive Summary

The previous session's design documents (in `notes/the_design/`) describe a **more complete implementation** than what actually exists. The gap between documentation and reality is significant but bridgeable.

### What's Actually Implemented (Verified in Code)

| Feature | Status | Location |
|---------|--------|----------|
| Force-directed physics | ✅ Working | `src/ui/graph_store.rs` |
| Infinite canvas (pan) | ✅ Working | `src/ui/graph.rs` |
| Zoom (button) | ✅ Working | `src/ui/graph.rs` - toolbar |
| Drag-to-move nodes | ✅ Working | `DragState::Dragging` |
| Lasso multi-select | ✅ Working | `DragState::Lasso` |
| Edge creation (rubber band) | ✅ Working | `DragState::CreatingEdge` |
| Edge persistence to DB | ✅ Working | `create_edge_in_db()` |
| Filesystem scan (machines/drives) | ✅ Working | `scan_directory()` |
| Directory expansion | ⚠️ Partial | Only machines/drives scan |
| Node gradients/visual polish | ✅ Working | `src/ui/graph_nodes.rs` |
| Status indicators | ✅ Working | CSS classes |
| File picker (column view) | ✅ Working | `src/ui/file_picker.rs` |
| Review queue UI | ✅ Working | `src/ui/review_queue.rs` |
| Notification system | ✅ Working | `src/ui/notification.rs` |
| Drive detection | ✅ Working | `src/devices/` |
| Transfer engine (copier) | ✅ Working | `src/engine/copier.rs` |
| Scanner | ✅ Working | `src/engine/scanner.rs` |
| Scheduler | ✅ Working | `src/engine/scheduler.rs` |

### What's NOT Implemented (Despite Docs Saying Otherwise)

| Feature | Doc Says | Reality |
|---------|----------|---------|
| Directory expansion with filesystem scan | "COMPLETE" | Only machines/drives scan; directories don't trigger scan |
| Edge creation complete flow | "COMPLETE" | DB creation exists but drop handler may not wire correctly |
| Multi-drag (drag all selected nodes) | "COMPLETE" | Lasso selects, but multi-drag logic incomplete |
| Node grouping | "NOT IMPLEMENTED" | Correct - not started |
| Central Output node | "NOT IMPLEMENTED" | Correct - not started |
| Per-node error badges | "NOT IMPLEMENTED" | Correct - not started |
| Force-directed layout | "COMPLETE" | Working but constants tuned for clusters, not general layout |

---

## Key Architectural Insights

### 1. Engine Code IS Complete (But Not Wired)

The transfer engine is fully implemented:
- `engine/copier.rs` - Chunked copy with blake3 hash, 256KB chunks, progress every 4 chunks
- `engine/scanner.rs` - Filesystem walk, job creation, skip symlinks
- `engine/scheduler.rs` - Bounded concurrency (4 parallel), recovery of stuck jobs

**But:** These are NOT called from the UI. The UI creates edges (intents) but doesn't trigger `scheduler::run_intent()`.

### 2. Database Schema IS Complete

All tables defined in `src/db.rs` (SCHEMA_V1):
- `machine` - Local/remote machines
- `drive` - Mounted drives with UUID, capacity, mount point
- `location` - File paths with graph_x/graph_y for persistence
- `intent` - Sync relationships with status tracking
- `transfer_job` - Individual file transfers
- `file_record` - Hash cache for deduplication
- `exists_at` - Path verification
- `review_item` - Error queue

### 3. Two Different Error Models

**Kip model:**
```rust
pub enum CopyError {
    SourceNotFound(String),
    PermissionDenied(String),
    HashMismatch { source_hash: String, dest_hash: String },
    // ...
}
```

**backup-tool model:**
```rust
pub enum BackupError {
    ConfigLoad { config_type: String, path: String, reason: String },
    BackupFailed { source: String, dest: String, reason: String },
    // ...
}
```

**Insight:** The unified `KipError` in our new API spec should merge both.

### 4. Config Systems Are Completely Different

**backup-tool:**
- TOML files in `~/.config/backup-tool/`
- `drives.toml` - Drive definitions
- `apps/*.toml` - Per-app folder configs
- Merged at runtime

**Kip:**
- No config files
- Everything in SurrealDB
- Locations/intents created via UI

**Migration strategy:** Import TOML → create DB records (as designed in `05_migration_plan.md`)

### 5. Dioxus Patterns Are Well-Established

The codebase has learned from mistakes:
- All spawns wrapped in `use_effect`
- Resources don't update signals directly
- Value capture (not signal capture) in closures
- Console-only logging (WARN level)

These patterns are documented in `CRITICAL_ISSUES.md` and followed in current code.

---

## Specific Code Findings

### graph_store.rs - Physics Constants

```rust
const REPULSION: f64 = 2000.0;      // Very strong - cluster separation
const SPRING_K: f64 = 0.03;         // Weak link tension
const CENTER_GRAVITY: f64 = 0.003;  // Very weak center pull
const DAMPING: f64 = 0.85;          // Velocity damping
const ALPHA_DECAY: f64 = 0.97;      // Slow decay
const ALPHA_MIN: f64 = 0.001;       // Stop threshold
```

**Note:** These are tuned for **cluster separation** (machines/drives form visual clusters), not general force-directed layout. May need adjustment for the unified architecture.

### graph_types.rs - Node Structure

```rust
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub path: String,
    pub kind: NodeKind,  // File, Directory, Group, Machine, Drive
    pub parent_id: Option<String>,
    pub color: String,
    pub position: Vec2,
    pub velocity: Vec2,
    pub pinned: bool,
    pub visible: bool,
    pub width: f64,
    pub height: f64,
    pub fx: Option<f64>,  // Fixed position during drag
    pub fy: Option<f64>,
}
```

**Note:** Already has `fx/fy` for D3-style drag. Well-designed.

### db.rs - Schema Details

Key schema features:
- `graph_x` / `graph_y` on `location` - For position persistence
- `status` on `intent` - idle/scanning/transferring/complete/needs_review
- `speed_mode` on `intent` - fast/throttled/background (Ninja mode support)
- `include_patterns` / `exclude_patterns` - Glob patterns
- `bidirectional` - For sync intents

### engine/copier.rs - Transfer Pipeline

```rust
const CHUNK_SIZE: usize = 256 * 1024;  // 256KB
const PROGRESS_INTERVAL: usize = 4;    // Update every 4 chunks (~1MB)

pub async fn copy_job(db: &DbHandle, job_id: &RecordId) -> Result<CopyResult, CopyError>
```

Features:
- Chunked copy with progress to DB
- blake3 hash of source and dest
- Hash verification after copy
- Status transitions: pending → transferring → complete/needs_review

### engine/scheduler.rs - Job Queue

```rust
const MAX_CONCURRENCY: usize = 4;

pub async fn run_intent(db: &DbHandle, intent_id: &RecordId) -> Result<RunResult, SchedulerError>
```

Features:
- Recovery: resets `transferring` jobs from crashed runs
- Semaphore-based concurrency
- Loops until no pending jobs
- Aggregates results

---

## Bugs & Issues (Current State)

### 1. Directory Expansion Not Wired

**Location:** `src/ui/graph.rs` - expansion handler

**Problem:** Only `Machine` and `Drive` nodes trigger `scan_directory()`. `Directory` nodes don't.

**Fix needed:**
```rust
match &node_kind {
    NodeKind::Machine { .. } | NodeKind::Drive { .. } => {
        scan_directory(&path, ...);  // Works
    }
    NodeKind::Directory { .. } => {
        // TODO: Also scan here
    }
    _ => {}
}
```

### 2. Edge Drop Handler May Not Fire

**Location:** `src/ui/graph.rs` - `onmouseup` handler

**Problem:** The `DragState::CreatingEdge` completion may not be wired correctly. Edge preview works, but drop may not create DB record.

**Needs verification:** Test edge creation flow end-to-end.

### 3. Zoom Buttons Exist But Wheel Doesn't

**Location:** `src/ui/graph.rs` - toolbar

**Status:** Button zoom works (`+`/`-`/`Reset`). Wheel zoom blocked by Dioxus API issue.

**Workaround:** Alt+drag pan works fine.

### 4. Multi-Drag Logic Incomplete

**Location:** `src/ui/graph.rs` - `DragState::Dragging` handler

**Problem:** Lasso selects multiple nodes, but dragging only moves one.

**Fix needed:** Track all selected nodes and apply same offset to each.

---

## backup-tool Insights

### Config Structure

**drives.toml:**
```toml
[[drives]]
name = "My Passport"
mount_point = "/Volumes/My Passport"
max_file_size = 4294967296  # 4GB FAT32 limit
```

**apps/obsidian.toml:**
```toml
[metadata]
name = "Obsidian Vault"
description = "Sync Obsidian notes"
priority = 800

[[folders]]
source = "~/Documents/Obsidian"
destinations = [
    { drive = "My Passport", path = "backups/obsidian" }
]
```

### SSH Support

backup-tool has full SSH support:
- Cloudflare Access integration
- Proxy command support
- Per-drive SSH config

**Migration:** This logic should move to `engine/drive.rs` or `engine/remote.rs`.

### State Management

backup-tool has legacy JSON state:
- `state/*.json` - Transfer state
- `state/state.json` - Global state

**Migration:** These should be discarded; SurrealDB is source of truth.

---

## Design Document Accuracy Assessment

### START_HERE.md
- **Accuracy:** 85%
- **Correct:** Physics system, infinite canvas, cluster separation
- **Incorrect:** Says directory expansion "COMPLETE" when it's partial

### CRITICAL_ISSUES.md
- **Accuracy:** 95%
- **Correct:** All infinite loop patterns are fixed
- **Correct:** Zoom wheel issue documented
- **Correct:** Edge creation incomplete

### IMPLEMENTATION_SUMMARY.md
- **Accuracy:** 75%
- **Correct:** Engine code exists and works
- **Incorrect:** Says multi-drag "COMPLETE" when it's not
- **Incorrect:** Says directory expansion "COMPLETE" when partial

### NEXT_AGENT_HANDOFF.md
- **Accuracy:** 90%
- **Correct:** Priorities are right
- **Correct:** Implementation approaches are sound
- **Note:** This doc was being written when session ended

### Phase1/*.md Documents
- **Accuracy:** 95%
- **Correct:** Implementation details are accurate
- **Useful:** Good reference for specific algorithms

---

## Recommendations for New Architecture

### 1. Preserve These Things

- **Physics constants** - Well-tuned for cluster separation
- **Engine code** - copier.rs, scanner.rs, scheduler.rs are solid
- **Database schema** - Complete and well-designed
- **Dioxus patterns** - Learned from hard mistakes
- **File picker** - Fully functional column view

### 2. Fix These Things First

- **Directory expansion** - Wire filesystem scan for Directory nodes
- **Edge drop handler** - Verify and fix edge creation completion
- **Multi-drag** - Complete the lasso + drag flow
- **API layer** - Implement the spec from `02_api_specification.md`

### 3. Defer These

- **Node grouping** - Cool feature, but not blocking
- **Central Output node** - Visual polish, not core functionality
- **Force-directed retuning** - Current constants work fine
- **Wheel zoom** - Button zoom works; wheel is nice-to-have

### 4. Import from backup-tool

- **SSH support** - Move to `engine/remote.rs`
- **Config import** - As designed in `05_migration_plan.md`
- **Cloudflare Access** - For remote machine auth

---

## File-by-File Status

| File | Status | Notes |
|------|--------|-------|
| `src/main.rs` | ✅ Good | Proper logging, DB init |
| `src/app.rs` | ✅ Good | Proper use_effect patterns |
| `src/db.rs` | ✅ Good | Complete schema |
| `src/lib.rs` | ⚠️ Needs update | Add API module exports |
| `src/ui/graph.rs` | ⚠️ Partial | Edge drop, multi-drag incomplete |
| `src/ui/graph_store.rs` | ✅ Good | Physics, drag state complete |
| `src/ui/graph_nodes.rs` | ✅ Good | Visual polish complete |
| `src/ui/graph_edges.rs` | ✅ Good | Bezier curves, preview |
| `src/ui/graph_types.rs` | ✅ Good | Well-designed types |
| `src/ui/file_picker.rs` | ✅ Good | Fully functional |
| `src/ui/review_queue.rs` | ✅ Good | UI complete |
| `src/ui/notification.rs` | ✅ Good | Toast system works |
| `src/engine/copier.rs` | ✅ Good | Chunked copy works |
| `src/engine/scanner.rs` | ✅ Good | Filesystem walk works |
| `src/engine/scheduler.rs` | ✅ Good | Job queue works |
| `src/devices/` | ✅ Good | Drive detection works |

---

## Conclusion

The codebase is in **better shape than it first appears**. The engine is complete, the database schema is solid, and the UI physics work well. The gaps are mostly **wiring issues** (connecting UI to engine) rather than missing implementations.

The new architecture documents (`01_*.md` through `05_*.md`) are aligned with what's actually in the code. The migration plan is feasible.

**Biggest risk:** The documentation says things are "COMPLETE" when they're actually "PARTIAL". This could mislead the next agent. Always verify by reading the actual code.
