# Unified Kip Architecture Overview

**Date:** February 21, 2026  
**Status:** Design Document

---

## Vision

Kip is a **file transfer orchestrator** — a unified system that manages intentional file mappings between locations (machines, drives, directories) with robust error handling, progress tracking, and visual representation.

The architecture unifies:
- **GUI Application** (Dioxus desktop app) — visual graph, interactive mapping, review queue
- **CLI Tool** (embedded in same codebase) — automation, scripting, headless operations
- **Backend Engine** (shared library) — transfer logic, scheduling, database operations

---

## Current State Assessment

### What Actually Works (Verified in Code)

1. **Force-Directed Physics** — `src/ui/graph_store.rs`
   - Repulsion: 2000.0 (strong cluster separation)
   - Link attraction: 0.03 spring constant
   - Center gravity: 0.003 (very weak)
   - Collision resolution with 3 iterations
   - Nodes spread infinitely, clusters stay separate

2. **Infinite Canvas** — `src/ui/graph.rs`
   - Alt+drag pan (1:1 viewport movement)
   - Button zoom (+/-/Reset, range 0.1x-5.0x)
   - Wheel zoom NOT working (Dioxus API issue)

3. **Node Interactions**
   - Drag-to-move with fx/fy (D3-style pinning)
   - Lasso selection (Shift+drag rectangle)
   - Multi-drag NOT fully working (lasso selects, but only one node moves)

4. **Edge Creation**
   - Ctrl/Alt+click starts edge
   - Rubber band preview line
   - Drop handler may NOT complete edge (needs verification)
   - `create_edge_in_db()` exists and works

5. **Filesystem Scanning**
   - Machine/Drive nodes: ✅ Scan works
   - Directory nodes: ⚠️ NOT wired (only machines/drives trigger scan)

6. **Transfer Engine** — `src/engine/`
   - `copier.rs`: Chunked copy (256KB), blake3 hash, progress to DB
   - `scanner.rs`: Filesystem walk, job creation, skip symlinks
   - `scheduler.rs`: Bounded concurrency (4), recovery, job queue
   - **NOT wired to UI** — engine runs but UI doesn't trigger it

7. **Database** — `src/db.rs`
   - Complete schema: machine, drive, location, intent, transfer_job, etc.
   - graph_x/graph_y for position persistence
   - Status tracking on intents

8. **UI Components**
   - File picker (column view): ✅ Complete
   - Review queue: ✅ UI complete
   - Notifications: ✅ Toast system works
   - Drive detection: ✅ `/Volumes/` polling

### What's Broken / Incomplete

1. **Directory Expansion** — Click on directory doesn't scan filesystem
2. **Edge Drop Completion** — May not create DB record on release
3. **Multi-Drag** — Lasso selects multiple, but drag moves only one
4. **Engine Wiring** — UI creates intents but doesn't run transfers
5. **Config Import** — backup-tool TOML configs not imported

### What backup-tool Has (That Kip Doesn't)

1. **TOML Config System** — `~/.config/backup-tool/`
   - `drives.toml` — Drive definitions
   - `apps/*.toml` — Per-app folder configs

2. **SSH Support** — Cloudflare Access, proxy commands

3. **CLI Interface** — clap-based commands

---

## Target Architecture (Phase B + C)

### Principle: Explicit API Layer

The API layer is **designed upfront**, not emerged incrementally. Both GUI and CLI consume the same API — neither touches the database directly.

### Layer Structure

```
┌─────────────────────────────────────────────────────────────┐
│                     Presentation Layer                       │
│  ┌─────────────────────┐    ┌─────────────────────────────┐ │
│  │   Kip GUI (Dioxus)  │    │   CLI (clap subcommands)    │ │
│  └─────────────────────┘    └─────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                      API Layer                               │
│  api::intent, api::transfer, api::location, api::review     │
│  api::query, api::config                                     │
├─────────────────────────────────────────────────────────────┤
│                    Backend Engine Core                       │
│  engine::transfer, engine::scanner, engine::scheduler       │
│  engine::drive, engine::review                              │
├─────────────────────────────────────────────────────────────┤
│                      Data Layer                              │
│  SurrealDB (local), TOML configs (import)                   │
└─────────────────────────────────────────────────────────────┘
```

### Key Architectural Decisions

1. **Single Crate, Multiple Binaries**
   - `kip/` is the root crate
   - `kip` binary — Dioxus GUI
   - `kip-cli` binary — CLI interface (new)
   - `lib.rs` — shared backend engine

2. **Database as Source of Truth**
   - All state lives in SurrealDB
   - Config files are **imported**, not primary storage
   - CLI and GUI are **views** onto the same data

3. **Intent-Centric**
   - All transfers flow through `Intent` records
   - Status: idle → scanning → transferring → complete/needs_review

4. **API Layer Explicit**
   - ~20 public functions defined in `02_api_specification.md`
   - Both GUI and CLI call same API
   - Engine modules don't expose DB directly

---

## Migration Strategy

### From backup-tool to kip

| backup-tool Feature | kip Equivalent | Migration Action |
|---------------------|----------------|------------------|
| `folders.toml` | `location` table | Import on first run |
| `drives.toml` | `drive` table | Import on first run |
| `state/*.json` | `transfer_job`, `intent` | Discard (DB is source of truth) |
| `run backup` | `api::run_intent()` | Create intent, trigger scan |
| `check backup` | `api::list_intents()` | Same query, different output |
| SSH support | `engine::remote` | Move logic to engine |

### From kip GUI to CLI

| kip GUI Feature | CLI Equivalent | Implementation |
|-----------------|----------------|----------------|
| File picker | `kip location add <path>` | Manual path specification |
| Edge drag | `kip intent create <src> <dst>` | Command syntax |
| Review queue | `kip review list` | Table output |
| Graph view | `kip status` | Text/tree output |

---

## File Structure (Target)

```
kip/
├── Cargo.toml              # Workspace root
├── src/
│   ├── main.rs             # GUI binary entry
│   ├── bin/
│   │   └── kip-cli.rs      # CLI binary entry
│   ├── lib.rs              # Library root
│   ├── api/
│   │   ├── mod.rs          # Public API surface
│   │   ├── intent.rs       # Intent CRUD
│   │   ├── transfer.rs     # Transfer operations
│   │   ├── location.rs     # Location management
│   │   ├── review.rs       # Review queue
│   │   ├── query.rs        # Read operations
│   │   └── config.rs       # Config import/export
│   ├── engine/
│   │   ├── mod.rs
│   │   ├── transfer.rs     # copier.rs → transfer engine
│   │   ├── scanner.rs      # Filesystem scanning
│   │   ├── scheduler.rs    # Job queue + concurrency
│   │   ├── drive.rs        # Drive detection
│   │   └── review.rs       # Error classification
│   ├── db/
│   │   ├── mod.rs          # Database initialization
│   │   ├── schema.rs       # SCHEMA_V1
│   │   └── models.rs       # SurrealValue types
│   └── ui/                 # GUI only (Dioxus components)
│       ├── app.rs
│       ├── graph.rs
│       └── ...
├── notes/
│   └── new_arch/           # This design directory
└── assets/
    └── main.css
```

---

## Next Steps

1. **Read `06_codebase_insights.md`** — Understand what's actually implemented
2. **Create CLI skeleton** — `src/bin/kip-cli.rs` with basic commands
3. **Extract backend engine** — Move `engine/copier.rs` into library
4. **Unify database code** — Single `db::init()` shared by both binaries
5. **Import backup-tool configs** — One-time migration on first run
6. **Wire UI to engine** — Connect graph actions to `api::*` calls

---

## Design Principles

1. **Backend First** — Logic lives in the engine, not the UI
2. **Shared State** — CLI and GUI are views onto the same database
3. **Explicit API** — Defined upfront, not emerged
4. **Preserve What Works** — Kip's visual model, backup-tool's SSH support
5. **Testable** — Backend engine has no GUI dependencies
