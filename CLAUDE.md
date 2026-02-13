# Kip

Kip (keep in place) is a file transfer orchestrator built in Rust with a Dioxus native UI. Born from a real emergency: 6 hours of manually babysitting 40+ rsync processes across USB drives and flaky SSH tunnels. Never again.

## The Core Idea

Kip is **intent-based**. The user says "these files should end up there" and Kip makes it happen — across reboots, drive disconnects, network drops. The only time Kip bothers the user is when it genuinely can't decide (conflict, permissions, disk full). Everything else resolves silently.

The primary UI is a **2D mapping graph**. Machines and drives are glass containers. Locations (files/dirs) are nodes inside them. Drawing an edge between two nodes creates an intent. That's the whole workflow.

## Design Docs (read before writing code)

1. `KIP_DESIGN_1.md` — Vision, core concepts, speed modes, principles
2. `KIP_DESIGN_2_DATA_MODEL.md` — SurrealDB schema, entities, graph relationships
3. `KIP_DESIGN_3_INTENT_LIFECYCLE.md` — State machine, triggers, concurrency
4. `KIP_DESIGN_4_ARCHITECTURE.md` — Menu bar app, thread model, copy pipeline
5. `KIP_DESIGN_5_ERROR_HANDLING.md` — Error classification, auto-resolve vs review
6. `KIP_DESIGN_6_MVP.md` — Phased roadmap, module structure, build order, **what's done vs. planned**
7. `KIP_DESIGN_7_MAPPING_GRAPH.md` — Graph UI, selection, grouping, Output node, status indicators
8. `KIP_DESIGN_8_FILE_PICKER.md` — Custom file picker (column view, drag-to-workspace, persistent panes)

## Decisions That Are Final

Do not revisit these:

- **SurrealDB 3.0** embedded with `kv-surrealkv`. Non-negotiable. See AGENTS.md for API gotchas.
- **Three speed modes**: Normal, Ninja, Blast. Ninja uses `setiopolicy_np(IOPOL_THROTTLE)`. Blast uses hill-climbing. Normal is default.
- **Menu bar app** (single process). Transfer engine in background threads. SurrealDB shared in-process.
- **blake3** for content hashing. Single-pass read → hash → write pipeline.
- **Location model**: always Machine/Drive + Path.
- **No Dioxus fullstack**. Desktop only.
- **Custom file picker** — not the OS native picker. Column view, glassmorphic, drag-to-workspace. See design doc 8.
- **Directories and groups are circles** in the graph. Files are pills/rectangles. Click a circle once to see children orbit around it. Click again to "enter" it.
- **iOS glassmorphism** throughout. `backdrop-filter: blur(24px)`, rgba backgrounds, Inter font, CSS variables.
- **Errors NEVER show in UI** unless user action is needed. Use `tracing` macros (`info!`, `error!`, etc.). Errors go to `kip.log`.

## Tech Stack

- Rust, Dioxus 0.7.3 desktop
- SurrealDB 3.0.0-beta.3 embedded (`kv-surrealkv`, NOT rocksdb)
- blake3 for hashing
- notify crate for filesystem watching
- DiskArbitration (macOS) for drive detection
- tokio async runtime

## Build & Run

```sh
dx build    # build (NOT cargo build)
dx serve --platform desktop   # run with hot reload (NOT cargo run)
```

**Never use `cargo build` or `cargo run`** — always use `dx build` / `dx serve`. Dioxus CLI does asset bundling and platform-specific setup that cargo alone misses.

## What AGENTS.md Is

`AGENTS.md` contains the technical reference for Dioxus 0.7 and SurrealDB 3.0 gotchas. Hard-won knowledge — read it before writing queries or RSX.

## Current State (what's built)

- SurrealDB embedded setup + idempotent schema
- Model structs for all entities
- Directory scanner, chunked copier, scheduler (engine stubs — code exists but not wired to UI)
- Drive detection via DiskArbitration polling
- **Mapping graph UI**: glass containers for machines/drives, location nodes with path containment, drag-to-connect edge creation (bezier curves), shift+click and lasso multi-select, status indicator, review queue
- **Add panel**: "+" button → pick machine/drive → opens file picker → creates location node
- **Remote machine creation**: inline form in add panel (name, hostname, SSH user)
- Glassmorphic CSS throughout
- Tracing-based logging to terminal + file

## What to Build Next (priority order)

1. **Custom file picker** — Replace `rfd` native picker with column-view picker. Drag files/dirs onto workspace. Persistent panes that minimize. See `KIP_DESIGN_8_FILE_PICKER.md`.
2. **Circular directory/group nodes** — Directories and groups render as circles. Click once = children orbit around. Click again = enter and show direct children.
3. **Grouping** — Select multiple nodes → group. Edge merging. Collapse/expand. See design doc 7.
4. **Central Output node** — Circular merge point at center of workspace.
5. **Per-node error badges** — Red/yellow circles at node top-left corners.
6. **Edge management** — Click to select, delete, view details.
7. **Node management** — Right-click context menu (delete, rename).
