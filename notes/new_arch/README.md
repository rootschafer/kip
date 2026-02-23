# New Architecture for Kip

**Created:** February 21, 2026  
**Updated:** February 21, 2026 (with codebase insights)

This directory contains the design documents for unifying Kip (GUI) and backup-tool (CLI) into a single application with a shared backend engine.

---

## Document Overview

### Reading Order

**Start here:**

1. **[06_codebase_insights.md](06_codebase_insights.md)** — **READ THIS FIRST**
   - Reality check: what's actually implemented vs. what docs claim
   - Verified feature status from code inspection
   - Specific bugs and gaps identified
   - File-by-file status assessment

2. **[01_architecture_overview.md](01_architecture_overview.md)** — Vision and target
   - Current state assessment (accurate)
   - Target architecture diagram
   - Migration strategy overview

3. **[02_api_specification.md](02_api_specification.md)** — The API contract
   - All public API functions
   - Data types and models
   - Error handling
   - **Authoritative** — implement to this spec

4. **[03_cli_design.md](03_cli_design.md)** — CLI command reference
   - Command hierarchy
   - Each command's syntax, options, output
   - Maps commands to API calls

5. **[04_backend_modules.md](04_backend_modules.md)** — Internal implementation
   - Module structure
   - How API functions are implemented
   - Engine module responsibilities

6. **[05_migration_plan.md](05_migration_plan.md)** — Step-by-step migration
   - 7 phases from current state to target
   - Includes Phase 6: Wire Engine (fix gaps)
   - File movements
   - Verification checklist

---

## Quick Reference

### Target File Structure

```
kip/
├── src/
│   ├── main.rs              # GUI binary
│   ├── bin/
│   │   └── kip-cli.rs       # CLI binary
│   ├── lib.rs               # Library root
│   ├── api/                 # Public API (6 modules)
│   ├── engine/              # Backend engine (5 modules)
│   ├── db/                  # Database layer
│   ├── config/              # Config import/export
│   └── ui/                  # GUI components
```

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| API layer explicit, not emergent | Clear contract for both GUI and CLI |
| Backend-first implementation | Logic in engine, not UI |
| Database as source of truth | CLI and GUI are views onto same data |
| Intent-centric model | All transfers flow through Intent |
| Single crate, multiple binaries | Shared code, unified versioning |

### API at a Glance

```rust
// Intents
api::create_intent(source, destinations, config) -> IntentId
api::run_intent(intent_id, progress_cb) -> RunResult
api::list_intents() -> Vec<IntentSummary>

// Locations
api::add_location(path, label, machine) -> LocationId
api::list_locations() -> Vec<LocationSummary>

// Review
api::list_review_items() -> Vec<ReviewItem>
api::resolve_review(review_id, resolution) -> Result<()>

// Config
api::import_backup_tool_config(config_dir) -> ImportResult
```

---

## For the Next AI Agent

### Your Task

Implement this architecture in the following order:

1. **Read `06_codebase_insights.md`** — Understand what's actually in the code
2. **Read `01_architecture_overview.md`** — Understand the target
3. **Start with Phase 1** (Foundation) from `05_migration_plan.md`
   - Create `src/bin/kip-cli.rs`
   - Create `src/api/` module structure
   - Get stubs compiling

4. **Then Phase 2** (Backend Extract)
   - Reorganize `engine/` modules
   - Rename `copier.rs` → `transfer.rs`

5. **Then Phase 3** (Unify Database)
   - Create `src/db/mod.rs`
   - Both binaries use same `db::init()`

6. **Then Phase 4** (Config Import)
   - Import backup-tool TOML configs

7. **Then Phase 5** (GUI Migration)
   - Update GUI to call API instead of direct DB

8. **Then Phase 6** (Wire Engine) — **CRITICAL**
   - Fix directory expansion (directories don't scan)
   - Verify edge drop handler
   - Fix multi-drag
   - Wire run intent button

9. **Then Phase 7** (Polish)
   - Tests, error handling, docs

### Important Constraints

- **Keep git history together** — All changes in `~/kip/`
- **Preserve what works** — Kip's visual model, backup-tool's SSH support
- **Test incrementally** — Verify each phase before proceeding
- **Update this documentation** — If you deviate from the plan, document why

### Build Commands

```bash
# Always use dx, not cargo
dx build                        # Build
dx serve --platform desktop     # Run GUI
```

### When in Doubt

1. Read `06_codebase_insights.md` for what's actually in the code
2. Read `02_api_specification.md` for the API contract
3. Read `05_migration_plan.md` for implementation steps
4. Read existing code in `src/engine/` for patterns

---

## Status

| Document | Status | Notes |
|----------|--------|-------|
| Codebase Insights | ✅ Complete | **START HERE** |
| Architecture Overview | ✅ Complete | Updated with accurate status |
| API Specification | ✅ Complete | Authoritative reference |
| CLI Design | ✅ Complete | All commands specified |
| Backend Modules | ✅ Complete | Implementation details |
| Migration Plan | ✅ Complete | 7 phases including Wire Engine |
| **Implementation** | ⏳ Not Started | Begin with Phase 1 |

---

## Related Documentation

### In This Repo
- `/Users/anders/kip/notes/the_design/` — Original Kip development plan
- `/Users/anders/kip/AGENTS.md` — Technical reference and gotchas
- `/Users/anders/kip/CLAUDE.md` — Project overview and decisions

### External
- `/Users/anders/.dotfiles/nix-ders/backup-tool/` — Original backup-tool codebase
- `/Users/anders/kip/external/nexus-node-sync/` — TypeScript/D3 reference implementation

---

## Summary of What Was Done

The previous session completed:
1. **Backup-tool SurrealDB integration** — Fixed dependency issues, API usage
2. **Kip transfer engine wiring** — Fixed RecordId API, Point2D handling
3. **Architecture design** — This document set

Both projects compile successfully. The architecture documents provide a complete blueprint for unification.

**Key insight from codebase exploration:** The engine is complete but not wired to the UI. The gaps are wiring issues, not missing implementations.
