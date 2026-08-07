# Kip — File Transfer Orchestrator

## Vision

A persistent, crash-resilient file transfer app. You declare intents ("these files should end up there") and Kip makes it happen — surviving reboots, network drops, drive disconnects. Errors resolve silently when possible; when they can't, they queue up for human review with clear resolution options.

Yes this is built with AI. I'm mentioning this because I don't want people to think I'm hiding it. I wanted this tool for myself, it would've never been worth it for me to build it myself: "No mon', no learn, no fun, no chance unless asked nicely" and nobody asked so my hands were tied. Any stable release will always be human-reviewed and development is always human-guided.

## How It Works

The primary UI is a **2D mapping graph**. Machines and drives appear as glass containers. Files and directories are nodes inside them. Draw an edge between two nodes and Kip keeps them in sync.

- **Intent-based**: "these files should be on that drive" — not "copy this file now"
- **Crash-resilient**: Survives reboots, drive pulls, network drops. Resumes where it left off.
- **Silent errors**: Auto-retries transient failures. Only bothers you when it genuinely can't decide.
- **Three speed modes**: Normal (balanced), Ninja (background, like Time Machine), Blast (max throughput with hill-climbing tuner — big red button)

## Core Features

### Mapping Graph
- Drag-to-connect: draw an edge between two location nodes to create a transfer intent
- Glassmorphic iOS-style visual design
- Path containment detection (nested directories are visually indented)
- Shift+click and lasso multi-select
- Custom file picker with column view — drag files/dirs from the picker directly onto the graph

### Transfer Engine
- Chunked file copy with blake3 hashing during transfer (single-pass pipeline)
- Per-job progress tracking, persisted to SurrealDB
- Resume on restart: incomplete jobs pick up where they left off
- Drive detection via DiskArbitration — auto-resumes when a drive reconnects

### Error Review Queue
- Retryable errors auto-retry with exponential backoff
- Non-retryable errors queue for human review with file previews and resolution options
- Conflict detection: same file, different content → side-by-side comparison

### File Index
- Every file Kip touches is recorded with a blake3 content hash
- Graph relationships: `file_record → exists_at → location`
- Enables duplicate detection across all machines/drives

## Tech Stack

- Rust, Dioxus 0.7.3 (desktop only)
- SurrealDB 3.0 embedded (`kv-surrealkv`)
- blake3 for content hashing
- DiskArbitration (macOS) for drive detection
- tokio async runtime

## Building

The GUI needs the Dioxus CLI (`dx`) for asset bundling:

```sh
dx build
dx serve --platform desktop
```

Everything else is plain cargo:

```sh
cargo build -p cli        # the `kip` command-line tool
cargo test --workspace    # test suite
```

Tests requiring an external service (SSH host, configured rclone remote) are
marked `#[ignore]`, so the default run needs no network. Run them with
`cargo test --workspace -- --ignored` once those services exist.

### Docker

Builds the full workspace and vendors every dependency, so the container runs
with no network:

```sh
docker build -t kip-dev .
docker run --rm --network none kip-dev
```

## Configuration

The CLI reads TOML from `~/.config/kip/` — `drives.toml` for destinations and
`apps/*.toml` for what to back up. Set `$KIP_CONFIG_DIR` to point elsewhere.
See `examples/drives-with-cloud.toml` for the shape of a drive config.

## Design Docs

Detailed design documentation lives in `notes/the_design/`:

1. `KIP_DESIGN_1.md` — Vision, core concepts, speed modes
2. `KIP_DESIGN_2_DATA_MODEL.md` — SurrealDB schema, entities, graph relationships
4. `KIP_DESIGN_4_ARCHITECTURE.md` — Menu bar app, thread model, copy pipeline
6. `KIP_DESIGN_6_MVP.md` — Phased roadmap, what's done vs. planned
7. `KIP_DESIGN_7_MAPPING_GRAPH.md` — Graph UI, selection, grouping, node types

(3, 5 and 8 were planned but never written.)

`notes/the_design/START_HERE.md` lists the open workstreams.
