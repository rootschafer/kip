# Ferry

Ferry is a file transfer orchestrator built in Rust with a Dioxus native UI. It was born from a real emergency: 6 hours of manually babysitting 40+ rsync processes across USB drives and flaky SSH tunnels, restarting failed transfers, tracking what went where, and praying nothing got missed before wiping a Mac. Never again.

## The Core Idea

Ferry is **intent-based**, not action-based. The user doesn't say "copy this file now." They say "these files should end up there" and Ferry makes it happen — across reboots, drive disconnects, network drops, whatever. The only time Ferry bothers the user is when it genuinely can't decide what to do (conflict, permissions, disk full). Everything else resolves silently.

Think of it as a transfer daemon with a review queue UI. Set it and forget it. Come back to either "done" or a short list of decisions.

## Design Docs (read these first)

The design is thorough. Read before writing code:

1. `FERRY_DESIGN_1.md` — Vision, core concepts, speed modes, principles
2. `FERRY_DESIGN_2_DATA_MODEL.md` — SurrealDB schema, entities, graph relationships, self-maintaining file index
3. `FERRY_DESIGN_3_INTENT_LIFECYCLE.md` — State machine (idle → scanning → transferring → complete), triggers, concurrency
4. `FERRY_DESIGN_4_ARCHITECTURE.md` — Menu bar app, thread model, copy pipeline, speed modes, device detection
5. `FERRY_DESIGN_5_ERROR_HANDLING.md` — Error classification, auto-resolve vs review, conflict detection, notification strategy
6. `FERRY_DESIGN_6_MVP.md` — Phased roadmap, module structure, build order

## Decisions That Are Final

Do not revisit these. They were discussed and settled:

- **SurrealDB** (embedded, not SQLite). Graph relationships for file-location tracking, LIVE SELECT for reactive UI. Non-negotiable.
- **Three speed modes**: Normal, Ninja, Blast. Not two, not five. Ninja uses `setiopolicy_np(IOPOL_THROTTLE)`. Blast uses a hill-climbing throughput tuner. Normal is the default with no special OS hints.
- **Menu bar app** (single process), not a daemon + separate GUI. The transfer engine runs in background threads. SurrealDB is shared in-process. LIVE SELECT connects engine to UI reactively.
- **blake3** for content hashing. Fast, streamable, computed during copy (single-pass read → hash → write pipeline).
- **Location model**: always Machine/Drive + Path. These are the two fundamental primitives.
- **No Dioxus fullstack**. This is a desktop app. See AGENTS.md for why.

## Tech Stack

- Rust, Dioxus 0.7.1 desktop
- SurrealDB embedded (kv-rocksdb)
- blake3 for hashing
- notify crate for filesystem watching
- DiskArbitration (macOS) for drive detection
- tokio async runtime

## Build Order (MVP)

Follow this sequence — each step builds on the last:

1. SurrealDB embedded setup + table definitions
2. Rust model structs (Intent, TransferJob, Location, Machine, Drive, FileRecord, ReviewItem)
3. Directory scanner (enumerate source → file manifest)
4. Chunked file copier (read → blake3 → write pipeline with progress)
5. Job scheduler (pull pending jobs, dispatch to copier, update status)
6. Basic Dioxus UI (intent list, progress bars, "new intent" button)
7. Drive detection (DiskArbitration mount/unmount callbacks)
8. Error review queue UI
9. Menu bar integration
10. Blast mode throughput tuner

## UI Personality

The Blast button should be red and look playfully dangerous. The rest of the UI should be clean and minimal — Ferry gets out of the way. The review queue is the hero feature: expandable cards with file previews, clear resolution buttons, batch actions for similar errors.

## What AGENTS.md Is

`AGENTS.md` contains the Dioxus 0.7 API reference. It is the source of truth for Dioxus syntax. Do not use old Dioxus patterns (`cx`, `Scope`, `use_state` — all gone in 0.7).
