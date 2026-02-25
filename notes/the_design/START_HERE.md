# Kip Development: Getting Started

**Date:** February 22, 2026

---

## Quick Start

### For New Developers

1. **Read this first:** `INTERACTION_MODEL.md` — How the UI works (and should work)
2. **Then read:** `COMPREHENSIVE_DEVELOPMENT_PLAN.md` — Current state and roadmap
3. **Architecture:** `new_arch/README.md` — API layer and CLI architecture

### For AI Agents

**DO NOT start coding until you have:**
1. Read `INTERACTION_MODEL.md` — Understand the interaction patterns
2. Read `COMPREHENSIVE_DEVELOPMENT_PLAN.md` — Know what's implemented vs. planned
3. Reviewed the code structure in `src/`

---

## Project Overview

**Kip** is a file transfer orchestrator. Users create sync relationships between locations (files, directories, machines) by connecting nodes in a 2D graph workspace.

**Tech Stack:**
- **Frontend:** Dioxus 0.7.3 (Rust, desktop + web capable)
- **Backend:** Rust library with API layer
- **Database:** SurrealDB 3.0.0 (stable) with SurrealKV embedded storage
- **CLI:** clap-based command interface

---

## Current State (February 22, 2026)

### ✅ What Works

| Feature | Status | Notes |
|---------|--------|-------|
| Database layer | ✅ Complete | SurrealDB 3.0.0 stable |
| API layer | ✅ Complete | `src/api/*` modules |
| CLI | ✅ Complete | Full command set |
| Transfer engine | ✅ Complete | Chunked copying, hashing |
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

```bash
# Always use dx, not cargo
dx build                        # Build desktop app
dx serve --platform desktop     # Run with hot reload
dx check                        # Check without building

# CLI
cargo build --bin kip-cli       # Build CLI
./target/debug/kip-cli --help   # Show CLI help

# Formatting
dx fmt                          # Format Dioxus code
cargo fmt                       # Format Rust code
```

---

## Key Directories

```
kip/
├── src/
│   ├── api/              # API layer (intent, location, review, etc.)
│   ├── engine/           # Transfer engine (transfer, scanner, scheduler)
│   ├── db/               # Database layer (schema, init)
│   ├── ui/               # Dioxus UI components
│   └── bin/              # CLI binary
├── crates/
│   └── actix-dioxus-serve/  # Web serving (future)
├── tests/                # Integration tests
└── notes/
    ├── the_design/       # Design documentation
    └── new_arch/         # Architecture documentation
```

---

## Design Documentation

### Core Documents

| Document | Purpose |
|----------|---------|
| `INTERACTION_MODEL.md` | Click/drag/keyboard behavior specification |
| `COMPREHENSIVE_DEVELOPMENT_PLAN.md` | Current state, roadmap, technical debt |
| `KIP_DESIGN_7_MAPPING_GRAPH.md` | Graph UI architecture (still relevant) |
| `KIP_DESIGN_8_FILE_PICKER.md` | File picker design |

### Architecture Documents

| Document | Purpose |
|----------|---------|
| `new_arch/README.md` | Entry point for architecture docs |
| `new_arch/01_architecture_overview.md` | Unified architecture vision |
| `new_arch/02_api_specification.md` | API layer specification |
| `new_arch/05_migration_plan.md` | Implementation phases |

### Historical Documents (Reference Only)

These documents contain outdated information but may have useful context:

- `KIP_DESIGN_1.md` through `KIP_DESIGN_6.md` — Early design thinking
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
# Run integration tests
cargo test --test integration_tests -- --test-threads=1

# Run unit tests
cargo test --test api_tests -- --test-threads=1
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

