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

**UI:**
- [x] Menu bar icon with transfer status
- [x] Main window: intent list with progress bars
- [x] "New intent" flow: pick source folder, pick destination, go
- [x] Review queue: list of errors with resolution buttons
- [x] Speed mode toggle (Normal / Blast)

**File index:**
- [x] Record every file with blake3 hash in SurrealDB
- [x] Track which locations have which files (exists_at graph)
- [ ] Duplicate detection UI (Phase 2 — data is collected though)

**NOT in MVP:**
- Remote machines (SSH)
- File preview
- Sync intents (only one-shot)
- Ninja mode
- 3D preview
- Duplicate detection UI
- Include/exclude glob patterns
- Scheduling

### Definition of Done:
You can drag a folder onto Kip, point it at a USB drive, and walk away. Come back to find everything copied, or a short list of problems with clear fix-it buttons. Pull the drive mid-copy, plug it back in, Kip picks up where it left off. Reboot your Mac, Kip resumes on login.

---

## Phase 2: "Remote + Ninja"

**Remote machines:**
- SSH/SFTP transfer backend
- Machine entity with connection config
- Online/offline detection
- Resume across network drops (chunked transfer with byte offset)
- rsync-like delta transfer (only send changed bytes) — or just file-level delta for simplicity

**Ninja mode:**
- `setiopolicy_np(IOPOL_THROTTLE)` on worker threads
- 1-2 concurrent jobs max

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

## Module Structure (MVP)

```
src/
├── main.rs                 # Entry point, Dioxus launch
├── app.rs                  # Root Dioxus component
├── db.rs                   # SurrealDB setup, migrations, queries
├── engine/
│   ├── mod.rs              # Transfer coordinator
│   ├── copier.rs           # Chunked file copy + blake3 pipeline
│   ├── scanner.rs          # Directory enumeration, delta computation
│   ├── tuner.rs            # Blast mode hill-climbing concurrency
│   └── scheduler.rs        # Job queue, priority ordering
├── devices/
│   ├── mod.rs
│   └── macos.rs            # DiskArbitration drive detection
├── models/
│   ├── mod.rs
│   ├── intent.rs           # Intent struct + lifecycle transitions
│   ├── job.rs              # TransferJob struct
│   ├── location.rs         # Location, Machine, Drive
│   ├── file_record.rs      # FileRecord + exists_at
│   └── review.rs           # ReviewItem + resolution types
├── ui/
│   ├── mod.rs
│   ├── menu_bar.rs         # Menu bar icon + quick menu
│   ├── dashboard.rs        # Main intent list + progress
│   ├── new_intent.rs       # Intent creation flow
│   ├── review_queue.rs     # Error review list
│   └── components/
│       ├── progress_bar.rs
│       ├── speed_toggle.rs
│       └── file_picker.rs
└── util/
    ├── hash.rs             # blake3 helpers
    └── fs.rs               # Filesystem utilities
```

## What to Build First (order)

1. **SurrealDB setup** (`db.rs`) — embedded instance, table definitions
2. **Models** — Rust structs that map to SurrealDB tables
3. **Scanner** — enumerate a directory into a file manifest
4. **Copier** — chunked copy + hash, single file
5. **Scheduler** — pull pending jobs, dispatch to copier, update status
6. **Basic UI** — intent list, progress bar, "add intent" button
7. **Device detection** — drive mount/unmount events
8. **Review queue** — display errors, resolution buttons
9. **Menu bar** — icon, status, quick actions
10. **Blast mode tuner** — hill-climbing concurrency
