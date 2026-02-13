# Kip — File Transfer Orchestrator

## Vision
A persistent, crash-resilient file transfer app. You declare intents ("these files should end up there") and Kip makes it happen — surviving reboots, network drops, drive disconnects. Errors resolve silently when possible; when they can't, they queue up for human review with full file previews and clear resolution options. Yes this is built with AI, I'm mentioning this because I don't want people to think I'm hiding it. I wanted this tool for myself, it would've never been worth it for me to build it myself: "No mon', no learn, no fun, no chance unless asked nicely" and nobody asked so my hands were tied 🤷‍♂️ Any stable release will always be human-reviewed and development is always human-guided.

## Core Architecture (This was written before a ton of development was done and needs to be updated, but as far as underlying goals, the below content is still accurate)

### 1. Intent Engine (daemon/service) (Redisigned completly, need to update this part of the README, it's not accurate at all)
- User declares transfer intents: source → destination, with priority
- Intents persist to SQLite (survives restarts)
- Chunked, resumable transfers (like rsync --partial but native)
- Detects drive connect/disconnect, network availability
- Runs as background service, wakes on relevant events
- Delta-aware: only moves what changed
- Multi-destination: same source can target flash drive + server + cloud simultaneously

### 2. Error Review Queue
- Transfers succeed silently (toast notification at most)
- Failures classified: retryable (network blip) vs. needs-human (permissions, disk full, conflict)
- Retryable errors auto-retry with backoff
- Human-needed errors queue in a review list
- Each error shows: what failed, why, file preview, and resolution options (retry, skip, rename, pick version, etc.)
- Side-by-side preview for conflicts

### 3. File Preview System (TODO: Low priority)
Wide format support including:
- Images: PNG, JPG, SVG, WebP, HEIC
- 3D models: STL, OBJ, GLTF (rendered via wgpu)
- Code/text: syntax highlighted (syntect)
- PDFs (pdfium)
- Audio waveforms
- Video thumbnails

### 4. Transfer Speed Modes (FINAL — three modes)

**Normal** (default)
- Reasonable balance, no special OS scheduling hints
- Moderate concurrency

**Ninja**
- macOS: `setiopolicy_np(IOPOL_THROTTLE)` — OS deprioritizes Kip I/O automatically
- Aggressively yields under load. When system is idle, OS naturally gives Kip full throughput
- Same mechanism Time Machine uses
- Sacrifices significant speed when computer is under reasonable load

**Blast**
- UI: big red button, playfully dangerous aesthetic
- Full I/O priority, auto-tuned concurrency
- Hill-climbing controller on throughput:
  1. Start with 1 stream, large buffer
  2. Add a stream, measure throughput
  3. Throughput increased? Try another. No? Back off.
  4. Re-probe every 30s as conditions change
- Finds actual optimal speed per destination type (USB, SSD, network, tunnel)

### 5. Transfer Dashboard UI
- Progress bars, throughput graphs, ETA per intent
- Mode toggle (Normal / Ninja / Blast)
- Review queue with expandable file previews
- Conflict resolution: side-by-side source vs dest preview

### 6. File Index & Deduplication (SurrealDB)
- Every file Kip touches gets recorded with a content hash
- Enables high-probability duplicate detection across all machines/drives
- Change detection: know when a file has been modified since last transfer
- Self-maintaining: no expiration-based deletion. Records clean up when files are confirmed gone from all known locations (or similar sensible policy — TBD)
- SurrealDB as the backing store
- Soon you will also be able to tell kip when you think a node (file, group, or directory) contains a duplicate, and kip will check, if it finds some it tells you, if it was wrong, which it shouldn't ever be, but if it is, then you can force it and overwrite one.


Machines/drives are first-class entities. When a drive connects or a machine comes online, Kip detects it and resumes any pending intents targeting that location.

## Key Principles
- Intent-based, not action-based ("this folder should be on that drive" vs "copy this file now")
- Survives anything: reboot, drive pull, network drop, tunnel timeout
- Errors resolve automatically when possible; human review only when necessary
- No unnecessary knobs or sliders — Kip figures out the optimal approach
- Preview as many file types as possible for informed conflict resolution
