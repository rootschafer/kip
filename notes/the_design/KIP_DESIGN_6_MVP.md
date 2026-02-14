# Kip MVP Roadmap

## Guiding Principle
Build the smallest thing that would have saved Anders 6 hours tonight.

---

## Phase 1: "Would Have Saved My Ass" (MVP)

The scenario: "I need to copy 45GB of app data to a USB drive and a remote server, and I need it to survive my laptop freezing, the drive getting pulled, and the SSH tunnel dying."

### What we build:

**Core engine:**
- [x] Intent creation: source path → destination path (local only for now)
- [x] File scanning with manifest (path, size, mtime)
- [x] Chunked file copy with blake3 hashing during transfer
- [x] Progress tracking per-job, persisted to SurrealDB
- [x] Resume on restart: query incomplete jobs, re-copy
- [x] Basic error handling: retry 3x, then queue for review

**Location model:**
- [x] Local paths (folder on this machine)
- [x] Removable drives (detect by UUID via DiskArbitration)
- [x] Auto-resume when drive reconnects

**Speed modes:**
- [x] Normal (default)
- [x] Blast (hill-climbing concurrency tuner)
- [ ] Ninja (Phase 2 — Normal is fine for MVP)

**UI — Mapping Graph (replaced original dashboard):**
- [x] Glassmorphic visual design (iOS-style, backdrop-filter blur, CSS variables)
- [x] Machine/drive containers with color-coded headers
- [x] Location nodes inside containers (files and directories)
- [x] Path containment detection with visual nesting (indented nodes)
- [x] Drag-to-connect edge creation (bezier curves, colored by intent status)
- [x] Native file picker for adding nodes (blue "+" button → pick machine → pick file)
- [x] Remote machine creation form (name, hostname, SSH user)
- [x] Shift+click multi-select
- [x] Shift+drag lasso selection
- [x] Global status indicator (green/red dot with review item count)
- [x] Review queue component (error list with resolution buttons)
- [x] Errors logged to tracing, never shown in UI
- [ ] Grouping (select nodes → group → collapse/expand)
- [ ] Central "Output" node (merge point)
- [ ] Per-node/group error badges
- [ ] Edge click/select/delete
- [ ] Node delete / context menu

**File index:**
- [x] Record every file with blake3 hash in SurrealDB
- [x] Track which locations have which files (exists_at graph)
- [ ] Duplicate detection UI (Phase 2 — data is collected though)

**NOT in MVP:**
- Remote machines (SSH transfer — containers can be added but transfer is local-only)
- File preview
- Sync intents (only one-shot)
- Ninja mode
- 3D preview
- Duplicate detection UI
- Include/exclude glob patterns
- Scheduling
- Force-directed graph layout

### Definition of Done:
You can drag a folder onto Kip, point it at a USB drive, and walk away. Come back to find everything copied, or a short list of problems with clear fix-it buttons. Pull the drive mid-copy, plug it back in, Kip picks up where it left off. Reboot your Mac, Kip resumes on login.

---

## Phase 2: "Remote + Ninja + Graph Polish"

**Remote machines:**
- SSH/SFTP transfer backend
- Machine entity with connection config
- Online/offline detection
- Resume across network drops (chunked transfer with byte offset)
- rsync-like delta transfer (only send changed bytes) — or just file-level delta for simplicity

**Ninja mode:**
- `setiopolicy_np(IOPOL_THROTTLE)` on worker threads
- 1-2 concurrent jobs max

**Graph features:**
- Grouping (select → group → collapse/expand, edge merging)
- Central "Output" node
- Per-node/group error badges at top-left corner
- Force-directed layout (with columnar seed for small graphs)
- Layout persistence (positions saved to SurrealDB)
- Edge click/select/delete
- Node context menu (delete, rename label, etc.)
- Edge state animations (pulse for active transfers)
- Color-coded edge gradients (source color → dest color)
- Finder drag-and-drop onto graph

**File preview (basics):**
- Images: PNG, JPG, WebP, SVG inline preview
- Text/code: syntax highlighted with syntect
- Show previews in review queue for conflict resolution

**Duplicate detection:**
- UI for browsing duplicates across locations
- "X files (Y GB) are duplicated between MacBook and SOMETHING"
- Suggest cleanup actions

**Include/exclude patterns:**
- Glob patterns per intent: `*.vscdb`, `node_modules/`, `.git/`

---

## Phase 3: "Polish + Power"

**Sync intents:**
- Filesystem watcher (notify crate) on source
- Change detection → automatic rescan → transfer
- Conflict handling for bidirectional sync

**3D file preview:**
- wgpu renderer for STL, OBJ, GLTF
- Interactive orbit/zoom in preview pane

**Advanced previews:**
- PDF rendering (pdfium)
- Audio waveform display
- Video thumbnail extraction

**Scheduling:**
- Cron-like schedule per intent ("sync every night at 2am")

**Multi-destination:**
- One intent, multiple destinations simultaneously
- Per-destination progress and error tracking

---

## Phase 4: "Platform"

**Cross-platform:**
- Linux support (inotify for file watching, udisks2 for drive detection)
- Windows support (ReadDirectoryChangesW, WMI for drives)

**Cloud destinations:**
- S3, GCS, Backblaze B2 as destination types
- Use existing cloud SDKs

**CLI:**
- `kip add /source /dest` — create intent from terminal
- `kip status` — show active transfers
- `kip review` — interactive error resolution

---

## Module Structure (current)

```
src/
├── main.rs                 # Entry point, Dioxus launch, tracing/logging setup
├── app.rs                  # Root Dioxus component (header + graph + review queue)
├── db.rs                   # SurrealDB setup, schema (DEFINE statements), bootstrap
├── engine/
│   ├── mod.rs
│   ├── copier.rs           # Chunked file copy + blake3 pipeline
│   ├── scanner.rs          # Directory enumeration, delta computation
│   ├── tuner.rs            # Blast mode hill-climbing concurrency
│   └── scheduler.rs        # Job queue, priority ordering
├── devices/
│   ├── mod.rs
│   └── macos.rs            # DiskArbitration drive detection (polls /Volumes/)
├── models/
│   ├── mod.rs
│   ├── intent.rs           # Intent, IntentStatus, IntentKind, SpeedMode
│   ├── job.rs              # TransferJob, JobStatus
│   ├── location.rs         # Location, Machine, Drive structs
│   ├── file_record.rs      # FileRecord + ExistsAt
│   └── review.rs           # ReviewItem, ErrorKind
├── ui/
│   ├── mod.rs
│   ├── graph.rs            # Mapping graph component (main workspace)
│   ├── graph_types.rs      # ContainerView, NodeView, EdgeView, containment logic
│   ├── review_queue.rs     # Error review list + resolution buttons
│   └── file_picker.rs      # Custom column-view file picker (TODO)
└── util/
    └── mod.rs
```

## What to Build Next (priority order)

1. **Custom file picker** — Column view, glassmorphic, drag files/dirs onto workspace, persistent panes that minimize to bottom tabs. Replaces `rfd` native picker. See `KIP_DESIGN_8_FILE_PICKER.md`.
2. **Circular directory/group nodes** — Directories and groups are circles, files are pills. Click circle once = children orbit around it. Click again = enter and show direct children inside.
3. **Grouping** — Select nodes → group → collapse/expand. Edge merging. See `KIP_DESIGN_7_MAPPING_GRAPH.md`.
4. **Central Output node** — Circular merge point at center of workspace.
5. **Per-node error badges** — Red/yellow badges at node top-left corners.
6. **Edge management** — Click to select, delete, view details.
7. **Node management** — Right-click context menu (delete, rename).
8. **Force-directed layout** — For larger graphs with many containers.
9. **Layout persistence** — Save container/node positions to SurrealDB.
