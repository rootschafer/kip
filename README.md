# Kip — File Transfer Orchestrator

Kip ("keep in place") is an intent-based file transfer tool. You declare where
files should end up and Kip works to make that true, resuming across reboots,
drive disconnects and network drops.

It exists because babysitting 40+ rsync processes across USB drives and flaky
SSH tunnels for six hours is a bad way to spend a night.

> Yes this is built with AI. I'm mentioning this because I don't want people to
> think I'm hiding it. I wanted this tool for myself, it would've never been
> worth it for me to build it myself: "No mon', no learn, no fun, no chance
> unless asked nicely" and nobody asked so my hands were tied. Any stable
> release will always be human-reviewed and development is always human-guided.

## Status

**Pre-alpha. Do not point this at data you cannot afford to lose.**

The project has two halves at very different maturity levels, and it is worth
being blunt about which is which.

### The CLI (`cli/`, binary `kip`) — usable

- TOML config: `drives.toml` for destinations, `apps/*.toml` for what to back up
- Backup and restore over **local drives** and **SSH**, via rsync
- State tracking, so an interrupted run resumes instead of restarting
- Cloud destinations through rclone — direct sync, or tar.gz-then-upload
- Dry-run mode, disk-space preflight, git-repo checks before backing up

Known gaps: cloud **restore** is a stub, and the cloud backup path currently
ignores the per-destination path so multiple folders collide in one remote
directory. See `notes/the_design/NEXTCLOUD_MIRROR_HANDOFF.md`.

### The GUI (`frontend/`) — prototype

A Dioxus desktop app showing a 2D mapping graph: machines and drives as glass
containers, files and directories as nodes, edges as transfer intents.

The graph renders, nodes drag, selection and the file picker work. It is **not**
wired to the transfer engine — drawing an edge does not currently move bytes.

### Designed but not built

These appear in the design docs and in enum definitions. They are not
implemented, and nothing in the code does what the names suggest:

| Feature | Reality |
|---|---|
| Speed modes (Normal/Ninja/Blast) | `enum` variants only — no I/O throttling, no hill-climbing tuner |
| Conflict detection / side-by-side compare | A `Conflict` review-reason variant; no comparison UI |
| Duplicate detection across drives | Not implemented |
| Auto-resume on drive reconnect | Drive detection exists; the resume hook does not |

What *is* real from the engine layer: blake3 content hashing in a chunked copy
pipeline (`daemon/src/engine/transfer.rs`), the directory scanner, and the
SurrealDB schema and models.

## Building

The workspace is ordinary cargo **except** the GUI, which needs the Dioxus CLI
(`dx`) for asset bundling.

```sh
cargo build -p cli          # the `kip` CLI
cargo test --workspace      # full test suite
```

For the GUI — note the `--package`, since this is a workspace with several
binaries and bare `dx build` cannot pick one:

```sh
dx build --package frontend
dx serve --package frontend --platform desktop    # hot reload
```

### Docker

Builds the whole workspace and vendors every crate, so the container runs with
no network at all:

```sh
docker build -t kip-dev .
docker run --rm --network none kip-dev            # runs the test suite
docker run --rm -it --network none kip-dev bash   # interactive
```

## Testing

```sh
cargo test --workspace              # everything that needs no external service
cargo test --workspace -- --ignored # SSH- and rclone-backed tests
```

Tests needing a live SSH host or a configured rclone remote are marked
`#[ignore]`, so the default run is hermetic and passes offline.

GUI components are tested headlessly by rendering them to HTML through Dioxus's
SSR renderer — no display server or browser required, so they run in the
container like everything else. See `frontend/tests/ui_render_tests.rs` and
`notes/the_design/UI_TESTING.md` for what that does and does not cover.

## Configuration

The CLI reads TOML from `~/.config/kip/` (override with `$KIP_CONFIG_DIR`):

- `drives.toml` — destinations. See `examples/drives-with-cloud.toml`.
- `apps/*.toml` — which folders to back up, and to which drives.

Fields that determine *where data is written* have no defaults. A drive missing
`mount_point`, or `host`/`user`/`path`, or `rclone_remote` is a hard error
naming the file and the key — Kip will not guess a destination and report a
backup as successful.

## Tech Stack

- Rust; Dioxus 0.7 (desktop)
- SurrealDB 3.0 embedded (`kv-surrealkv`)
- blake3 for content hashing
- rsync and rclone as transfer backends
- DiskArbitration (macOS) for drive detection
- tokio

The GUI is desktop-only. The `web` feature in `frontend/Cargo.toml` does not
build — the UI links the SurrealDB-backed data layer directly, which pulls
native tokio and cannot target wasm.

## Design Docs

In `notes/the_design/`:

1. `KIP_DESIGN_1.md` — Vision, core concepts, speed modes
2. `KIP_DESIGN_2_DATA_MODEL.md` — SurrealDB schema, entities, graph relationships
4. `KIP_DESIGN_4_ARCHITECTURE.md` — Thread model, copy pipeline
6. `KIP_DESIGN_6_MVP.md` — Phased roadmap, done vs. planned
7. `KIP_DESIGN_7_MAPPING_GRAPH.md` — Graph UI, selection, grouping, node types

Design docs 3, 5 and 8 were relocated rather than deleted:
`Phase2/Phase2.2_Intent_Lifecycle_Management.md` (intent lifecycle),
`Phase2/Phase2.3_Error_Handling_and_Review_Queue.md` (error handling),
`Phase1/Phase1.1_Directory_Expansion_and_File_Picker.md` (file picker).

`notes/the_design/START_HERE.md` lists the open workstreams.
