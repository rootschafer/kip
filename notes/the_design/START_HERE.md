# Kip Development: Getting Started

**Date:** February 22, 2026 (updated 2026-06-30)

---

## Open workstreams

Two threads are in flight; neither blocks the other.

- **Cloud mirror mode** — back up directly into a cloud remote that mirrors the
  local filesystem layout. Design is settled, implementation is partly done and
  has a known bug. See `NEXTCLOUD_MIRROR_HANDOFF.md`.
- **GUI interaction model** — see `NEXT_AGENT_HANDOFF.md`.

---

## Quick Start

### For New Developers

1. **Read this first:** `INTERACTION_MODEL.md` — How the UI works (and should work)
2. **Then read:** `COMPREHENSIVE_DEVELOPMENT_PLAN.md` — Current state and roadmap
3. **Architecture:** `new_arch/README.md` — API layer and CLI architecture

### For AI Agents

**DO NOT start coding until you have:**
1. Read the handoff for whichever workstream you're on (see "Open workstreams" above)
2. Read `INTERACTION_MODEL.md` — how the UI is meant to behave
3. Reviewed the relevant code: `cli/src/backup.rs`, `cli/src/folder.rs`, and
   `crates/kip-rclone/` for the cloud path; `frontend/src/` for the GUI

---

## Project Overview

**Kip** is a file transfer orchestrator. Users create sync relationships between locations (files, directories, machines) by connecting nodes in a 2D graph workspace.

**Tech Stack:**
- **Frontend:** Dioxus 0.7.10 (Rust, desktop only — the `web` feature does not build)
- **Backend:** Rust library with API layer
- **Database:** SurrealDB 3.0.0 (stable) with SurrealKV embedded storage
- **CLI:** clap-based command interface

---

## Current State (February 22, 2026)

### ✅ What Works

| Feature | Status | Notes |
|---------|--------|-------|
| Database layer | ✅ Complete | SurrealDB 3.0.0 stable |
| API layer | ✅ Complete | `frontend/src/api/*` modules |
| CLI | ✅ Complete | Full command set |
| Transfer engine | ⚠️ Built, not wired | `daemon/src/engine/` — the GUI does not call it |
| Filesystem scanner | ✅ Complete | Handles symlinks |
| Job scheduler | ✅ Complete | Bounded concurrency |
| Node rendering | ✅ Complete | Files=pills, dirs=circles |
| Force-directed layout | ✅ Complete | Cluster separation |
| Edge creation | ✅ Complete | Drag to connect |
| Lasso selection | ✅ Complete | Area select |
| Multi-drag | ✅ Complete | Move multiple nodes |
| File picker | ✅ Complete | Column navigation |

### ⚠️ What Needs Work

| Feature | Issue | Priority |
|---------|-------|----------|
| Click behavior | Single click selects AND drags (conflict) | HIGH |
| Context menus | Not implemented | HIGH |
| Keyboard shortcuts | Not implemented | HIGH |
| Orbit view | Partially working | MEDIUM |
| Enter view | Not implemented | MEDIUM |
| Node grouping | Not implemented | LOW |
| Layout persistence | Not implemented | LOW |

---

## Build Commands

Only the GUI needs `dx`; everything else is plain cargo. `--package` is
required — this is a workspace with three binaries, and bare `dx build` fails
with "Failed to find binary package to build".

```bash
# GUI (frontend crate)
dx build --package frontend
dx serve --package frontend --platform desktop   # hot reload

# CLI (package `cli`, binary `kip`)
cargo build -p cli
./target/debug/kip --help

# Everything
cargo test --workspace
cargo fmt
```

---

## Key Directories

```
kip/
├── cli/          # `kip` CLI — backup/restore over local, SSH, cloud
├── daemon/       # DB layer, transfer engine, scanner, graph store
│   └── src/engine/   # transfer.rs, scanner.rs, scheduler
├── frontend/     # Dioxus desktop GUI
│   ├── src/api/      # API layer (intent, location, review, query, …)
│   ├── src/ui/       # Components (graph, file picker, review queue)
│   └── tests/        # Includes headless SSR render tests
├── kip-core/     # Shared models and graph types
├── crates/
│   ├── kip-rsync/    # rsync wrapper (local + SSH)
│   └── kip-rclone/   # rclone wrapper (cloud)
├── docs/         # User-facing docs
├── examples/     # Example configs
└── notes/
    ├── the_design/   # Design documentation
    └── new_arch/     # Architecture documentation
```

---

## Design Documentation

### Core Documents

| Document | Purpose |
|----------|---------|
| `INTERACTION_MODEL.md` | Click/drag/keyboard behavior specification |
| `COMPREHENSIVE_DEVELOPMENT_PLAN.md` | Current state, roadmap, technical debt |
| `KIP_DESIGN_7_MAPPING_GRAPH.md` | Graph UI architecture (still relevant) |

### Architecture Documents

| Document | Purpose |
|----------|---------|
| `new_arch/README.md` | Entry point for architecture docs |
| `new_arch/01_architecture_overview.md` | Unified architecture vision |
| `new_arch/02_api_specification.md` | API layer specification |
| `new_arch/05_migration_plan.md` | Implementation phases |

### Historical Documents (Reference Only)

These documents contain outdated information but may have useful context:

- `KIP_DESIGN_1.md`, `_2_`, `_4_`, `_6_`, `_7_` — Early design thinking. Docs 3, 5 and 8 were moved into `Phase2/Phase2.2_`, `Phase2/Phase2.3_` and `Phase1/Phase1.1_` respectively.
- `Phase1/` through `Phase4/` — Original phase plans (superseded)
- `NEXT_AGENT_HANDOFF.md` — Previous handoff notes

---

## Critical Issues

See `CRITICAL_ISSUES.md` for known bugs and workarounds.

**Top issues:**
1. SurrealDB type coercion (RecordId vs String) — Partially fixed
2. Click/drag conflict — Needs interaction refactor
3. No context menus — Needs implementation

---

## Testing

```bash
# Whole workspace (hermetic — no network needed)
cargo test --workspace

# Frontend only
cargo test -p frontend --test integration_tests -- --test-threads=1
cargo test -p frontend --test api_tests -- --test-threads=1

# Headless GUI component rendering (no display required)
cargo test -p frontend --test ui_render_tests
```

**Note:** Some tests are ignored due to SurrealDB type issues.

---

## Development Workflow

1. **Pick a task** from `COMPREHENSIVE_DEVELOPMENT_PLAN.md`
2. **Read relevant docs** in `notes/the_design/`
3. **Implement** the feature
4. **Test** with `dx check` and `cargo test`
5. **Update docs** if behavior changes

---

## Getting Help

- **Architecture questions:** Read `new_arch/` documents
- **UI questions:** Read `KIP_DESIGN_7_MAPPING_GRAPH.md`
- **Interaction questions:** Read `INTERACTION_MODEL.md`
- **Bug troubleshooting:** Check `CRITICAL_ISSUES.md`

---

## Document History

| Date | Change |
|------|--------|
| 2026-02-13 | Initial version |
| 2026-02-17 | Updated with critical issues |
| 2026-02-22 | Major revision: accurate current state, new interaction model |

