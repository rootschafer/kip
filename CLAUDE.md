@AGENTS.md

# Kip

Kip (keep in place) is a file transfer orchestrator built in Rust with a Dioxus native UI. Born from a real emergency: 6 hours of manually babysitting 40+ rsync processes across USB drives and flaky SSH tunnels. Never again.

## The Core Idea

Kip is **intent-based**. The user says "these files should end up there" and Kip makes it happen — across reboots, drive disconnects, network drops. The only time Kip bothers the user is when it genuinely can't decide (conflict, permissions, disk full). Everything else resolves silently.

The primary UI is a **2D mapping graph**. Machines and drives are glass containers. Locations (files/dirs) are nodes inside them. Drawing an edge between two nodes creates an intent. That's the whole workflow.

## Design Docs (read before writing code)

All live under `notes/the_design/`:

1. `KIP_DESIGN_1.md` — Vision, core concepts, speed modes, principles
2. `KIP_DESIGN_2_DATA_MODEL.md` — SurrealDB schema, entities, graph relationships
4. `KIP_DESIGN_4_ARCHITECTURE.md` — Menu bar app, thread model, copy pipeline
6. `KIP_DESIGN_6_MVP.md` — Phased roadmap, module structure, build order, **what's done vs. planned**
7. `KIP_DESIGN_7_MAPPING_GRAPH.md` — Graph UI, selection, grouping, Output node, status indicators

Three of the original eight were **relocated**, not deleted — look here, not for
a `KIP_DESIGN_3/5/8`:

| Original | Now lives at |
|---|---|
| 3 — Intent lifecycle | `Phase2/Phase2.2_Intent_Lifecycle_Management.md` |
| 5 — Error handling | `Phase2/Phase2.3_Error_Handling_and_Review_Queue.md` |
| 8 — File picker | `Phase1/Phase1.1_Directory_Expansion_and_File_Picker.md` |

Current workstreams and their handoffs are listed in
`notes/the_design/START_HERE.md`.

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

- Rust, Dioxus 0.7 desktop
- SurrealDB 3.0 embedded (`kv-surrealkv`, NOT rocksdb)
- blake3 for hashing
- notify crate for filesystem watching
- DiskArbitration (macOS) for drive detection
- tokio async runtime

## Build & Run

**The `frontend` crate (the Dioxus GUI) must be built with `dx`**, not cargo —
the Dioxus CLI does asset bundling and platform setup that cargo alone misses.
`--package` is required: this is a workspace with three binaries and bare
`dx build` fails with "Failed to find binary package to build".

```sh
dx build --package frontend
dx serve --package frontend --platform desktop    # hot reload
```

**Everything else is plain cargo.** The CLI, the daemon, and the library crates
have no asset pipeline, and the test suite is ordinary `cargo test`:

```sh
cargo build -p cli             # the `kip` CLI
cargo test --workspace         # full test suite
cargo test -p kip-rsync        # one crate
```

Tests that need external services (an SSH host, a configured rclone remote) are
marked `#[ignore]`; `cargo test` skips them and passes with no network. Run them
deliberately with `-- --ignored` once the service exists.

**Testing UI changes headlessly:** graph components are rendered to HTML with
`dioxus-ssr` and asserted on — no display, no browser, no wasm. See
`frontend/tests/ui_render_tests.rs`. This reaches layout geometry and state-to-
markup wiring, but not pointer/drag interaction. Read
`notes/the_design/UI_TESTING.md` before assuming a UI bug is verifiable here —
and note that a web/Playwright build is **not** available (`frontend` links
SurrealDB and native tokio, so it cannot target wasm).

### In Docker

`Dockerfile` builds the whole workspace and pre-fetches every crate, so the
container runs with no network at all:

```sh
docker build -t kip-dev .
docker run --rm --network none kip-dev              # runs cargo test --workspace
docker run --rm -it --network none kip-dev bash     # poke around
```

`CARGO_NET_OFFLINE=true` is set in the image, so any accidental attempt to reach
the network fails immediately instead of hanging.

## What AGENTS.md Is

`AGENTS.md` contains the technical reference for Dioxus 0.7 and SurrealDB 3.0 gotchas. Hard-won knowledge — read it before writing queries or RSX.

## Current State (what's built)

- SurrealDB embedded setup + idempotent schema
- Model structs for all entities
- Directory scanner, chunked copier, scheduler (engine stubs — code exists but not wired to UI)
- Drive detection via DiskArbitration polling
- **Mapping graph UI**: glass containers for machines/drives, location nodes with path containment, drag-to-connect edge creation (bezier curves), shift+click and lasso multi-select, status indicator, review queue
- **Add panel**: "+" button → pick machine/drive → opens custom file picker
- **Custom file picker** ✅ DONE: Column-view with persistent panes, minimize/restore tabs, dir traversal, "Add to workspace" button
- **Remote machine creation**: inline form in add panel (name, hostname, SSH user)
- **Circular directory nodes** ✅ NEARLY COMPLETE: Directories render as circles with child counts, click once for orbit view, click again for expanded view
- Glassmorphic CSS throughout
- Tracing-based logging to terminal + file

### CLI (`cli/`, binary name `kip`)

Separate from the GUI and further along for actual transfers:

- TOML config in `~/.config/kip/` (`drives.toml` + `apps/*.toml`), overridable
  with `$KIP_CONFIG_DIR`
- Backup/restore over local drives and SSH, with state tracking for resume
- Cloud destinations via rclone (`crates/kip-rclone`) — direct sync and
  tar.gz-then-upload. **Known bug:** the per-destination path is ignored, so
  every folder lands in the same remote root. See
  `notes/the_design/NEXTCLOUD_MIRROR_HANDOFF.md`.
- Cloud *restore* is a stub
- `crates/kip-rsync` wraps rsync (local + SSH) with progress parsing

Drive configuration is strict: a drive missing the fields that determine where
data is written (`mount_point`, `host`/`user`/`path`, `rclone_remote`) is a hard
error naming the file and key. Never reintroduce a fallback like `localhost` or
a default user — a guessed destination sends a backup somewhere wrong and
reports success.

## What to Build Next (priority order)

### CLI

1. **Fix the cloud destination path bug** — per-destination paths are dropped;
   see `notes/the_design/NEXTCLOUD_MIRROR_HANDOFF.md` Phase A.
2. **Cloud mirror mode** — `mirror = true` on a destination. Same doc, Phase B.
3. **Cloud restore** — currently a stub in `cli/src/restore.rs`.

### GUI

1. ✅ **Custom file picker** — DONE. Column-view picker with persistent panes.
2. **Circular directory/group nodes** — Directories and groups render as circles. Click once = children orbit around. Click again = enter and show direct children. See `notes/plans/circular_nodes_implementation_plan.md` and `notes/CIRCULAR_NODES_PROGRESS.md`.
3. **Grouping** — Select multiple nodes → group. Edge merging. Collapse/expand. See design doc 7.
4. **Central Output node** — Circular merge point at center of workspace.
5. **Per-node error badges** — Red/yellow circles at node top-left corners.
6. **Edge management** — Click to select, delete, view details.
7. **Node management** — Right-click context menu (delete, rename).
