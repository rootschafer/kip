# Kip — File Transfer Orchestrator

## Vision
A persistent, crash-resilient file transfer app. You declare intents ("these files should end up there") and Kip makes it happen — surviving reboots, network drops, drive disconnects. Errors resolve silently when possible; when they can't, they queue up for human review with full file previews and clear resolution options.

The user should never think about errors unless Kip genuinely can't decide what to do.

## Core Architecture

### 1. Mapping Graph (primary UI)
The main interface is a 2D graph workspace. See `KIP_DESIGN_7_MAPPING_GRAPH.md` for full details:
- Machines and drives appear as glass containers
- Locations (files/directories) are nodes inside containers
- Drawing an edge between nodes creates a transfer intent
- Selection, grouping, and an Output merge-point provide organizational power
- iOS glassmorphic visual design throughout

### 2. Transfer Engine (background)
- Intent-based: user declares what should be where, Kip makes it happen
- Chunked, resumable transfers with blake3 hashing during copy (single-pass pipeline)
- Detects drive connect/disconnect, network availability
- Runs in background threads within the same process
- Delta-aware: only moves what changed
- Multi-destination: same source can target multiple locations simultaneously

### 3. Error Review Queue
- Transfers succeed silently (toast notification at most)
- Failures classified: retryable (network blip) vs. needs-human (permissions, disk full, conflict)
- Retryable errors auto-retry with backoff
- Human-needed errors queue in a review list with file previews and resolution options
- See `KIP_DESIGN_5_ERROR_HANDLING.md` for full error taxonomy

### 4. File Preview System (future)
Wide format support for informed conflict resolution:
- Images: PNG, JPG, SVG, WebP, HEIC
- 3D models: STL, OBJ, GLTF (rendered via wgpu)
- Code/text: syntax highlighted (syntect)
- PDFs (pdfium), audio waveforms, video thumbnails

### 5. Transfer Speed Modes (FINAL — three modes)

**Normal** (default)
- Reasonable balance, no special OS scheduling hints
- Moderate concurrency

**Ninja**
- macOS: `setiopolicy_np(IOPOL_THROTTLE)` — OS deprioritizes Kip I/O automatically
- Same mechanism Time Machine uses
- Aggressively yields under load; full throughput when system is idle

**Blast**
- UI: big red button, playfully dangerous aesthetic
- Full I/O priority, auto-tuned concurrency
- Hill-climbing controller: start with 1 stream, probe adding/removing, find optimal per-destination
- Re-probes every 30s as conditions change

### 6. File Index & Deduplication (SurrealDB)
- Every file Kip touches gets recorded with a blake3 content hash
- Enables high-probability duplicate detection across all machines/drives
- Change detection: know when a file has been modified since last transfer
- Self-maintaining: records clean up when files are confirmed gone from all known locations
- Users can also tell Kip when they think a node contains a duplicate — Kip checks and reports

## Key Principles
- Intent-based, not action-based ("this folder should be on that drive" vs. "copy this file now")
- Survives anything: reboot, drive pull, network drop, tunnel timeout
- Errors resolve automatically when possible; human review only when necessary
- Errors NEVER show in the UI unless user action is required — use tracing for everything else
- No unnecessary knobs or sliders — Kip figures out the optimal approach
- Preview as many file types as possible for informed conflict resolution
