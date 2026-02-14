# Kip Architecture

## Process Model: Menu Bar App

Kip runs as a single-process **menu bar application**. Not a separate daemon. Not a window you have to keep open.

```
┌──────────────────────────────────────────────┐
│              Kip Process                    │
│                                               │
│  ┌─────────────┐   ┌──────────────────────┐  │
│  │  Menu Bar    │   │  Transfer Engine     │  │
│  │  Icon +      │   │  (background threads)│  │
│  │  Status      │   │                      │  │
│  └──────┬──────┘   │  - Job scheduler     │  │
│         │           │  - File copier       │  │
│         │ click     │  - Hash computer     │  │
│         ▼           │  - Device watcher    │  │
│  ┌──────────────┐   │                      │  │
│  │  Main Window │   └──────────┬───────────┘  │
│  │  (Dioxus UI) │              │              │
│  │              │◄─────────────┘              │
│  │  - Dashboard │   SurrealDB LIVE SELECT     │
│  │  - Intents   │   (reactive updates)        │
│  │  - Reviews   │                             │
│  │  - Previews  │                             │
│  └──────────────┘                             │
│                                               │
│  ┌──────────────────────────────────────┐     │
│  │  SurrealDB (embedded, on-disk)       │     │
│  │  ~/Library/Application Support/Kip/ │     │
│  └──────────────────────────────────────┘     │
└──────────────────────────────────────────────┘
```

### Why single process?
- No IPC to design. Transfer engine and UI share the same SurrealDB instance.
- LIVE SELECT gives the UI reactivity without polling or message passing.
- Menu bar presence means it's "always running" without feeling like a window you need to manage.
- If the user force-quits: on next launch, Kip reads persisted state and resumes. Same resilience as a daemon, simpler architecture.

### Why NOT a daemon?
- launchd daemon + separate GUI = IPC layer (Unix socket, gRPC, etc). Complexity for no user benefit.
- The menu bar app *is* the daemon. It launches at login, lives in the menu bar, does its job.
- If we ever need to split (e.g., for a CLI tool that talks to Kip), we extract the engine into a library. But not now.

## Menu Bar Behavior

### Icon States
- **Idle**: Static kip/boat icon
- **Transferring**: Animated icon (subtle pulse or progress indicator)
- **Needs Review**: Badge with count (like mail unread count)
- **Error**: Red dot

### Click Behavior
- **Left click**: Open/toggle main window
- **Right click**: Quick menu
  - Current speed mode toggle (Normal / Ninja / Blast)
  - Pause all / Resume all
  - "3 items need review" → opens review queue
  - Quit

## Transfer Engine

### Thread Model
```
Main Thread (Dioxus UI + event loop)
  │
  ├── Transfer Coordinator Thread
  │     Owns the job scheduler. Pulls jobs from SurrealDB,
  │     assigns to worker threads, manages concurrency limits.
  │     │
  │     ├── Worker Thread 1 (file copy + hash)
  │     ├── Worker Thread 2
  │     ├── Worker Thread N (N = concurrency limit)
  │     │
  │     └── Verifier Thread (post-transfer hash check)
  │
  ├── Device Watcher Thread
  │     macOS: DiskArbitration framework callbacks
  │     Detects drive mount/unmount, updates SurrealDB
  │
  └── File Watcher Thread (for sync intents)
        notify crate, watches source directories
        On change: marks intent for rescan
```

### File Copy Pipeline (per job)

```
Source File
    │
    ▼
┌──────────┐     ┌──────────┐     ┌──────────┐
│  Read    │────▶│  Hash    │────▶│  Write   │
│  chunk   │     │  update  │     │  chunk   │
│  (64KB)  │     │  (blake3)│     │  to dest │
└──────────┘     └──────────┘     └──────────┘
    │                                   │
    │         Progress update           │
    └──────── bytes_transferred ────────┘
                    │
                    ▼
               SurrealDB
           (periodic flush)
```

Read, hash, and write happen in a single pass. No re-reading for verification — the hash is computed during the copy. After the copy, if verification is desired, read the destination and compare.

Chunk size:
- Local/USB: 256KB–1MB (larger = fewer syscalls, faster)
- Network: 64KB (fits in TCP window, responsive progress)

### Speed Mode Implementation

```rust
pub enum SpeedMode {
    Normal,
    Ninja,
    Blast,
}

impl SpeedMode {
    /// Set the I/O policy for the current thread
    fn apply_io_policy(&self) {
        match self {
            SpeedMode::Ninja => {
                // macOS: background I/O priority
                // The OS itself throttles us when other apps need I/O
                unsafe {
                    setiopolicy_np(
                        IOPOL_TYPE_DISK,
                        IOPOL_SCOPE_THREAD,
                        IOPOL_THROTTLE,
                    );
                }
            }
            SpeedMode::Normal => {
                // Default I/O priority, moderate concurrency
                unsafe {
                    setiopolicy_np(
                        IOPOL_TYPE_DISK,
                        IOPOL_SCOPE_THREAD,
                        IOPOL_DEFAULT,
                    );
                }
            }
            SpeedMode::Blast => {
                // Normal I/O priority + aggressive concurrency tuning
                unsafe {
                    setiopolicy_np(
                        IOPOL_TYPE_DISK,
                        IOPOL_SCOPE_THREAD,
                        IOPOL_NORMAL,
                    );
                }
            }
        }
    }

    fn base_concurrency(&self) -> usize {
        match self {
            SpeedMode::Ninja => 1,
            SpeedMode::Normal => 4,
            SpeedMode::Blast => 1, // starts at 1, hill-climber adjusts
        }
    }
}
```

### Blast Mode Hill Climber

```rust
struct ThroughputTuner {
    current_concurrency: usize,
    last_throughput: f64,      // bytes/sec
    direction: i8,             // +1 = increasing, -1 = decreasing
    stable_count: usize,       // how many probes since last change
    probe_interval: Duration,  // 30 seconds
}

impl ThroughputTuner {
    fn probe(&mut self, current_throughput: f64) -> usize {
        if current_throughput > self.last_throughput * 1.05 {
            // Going faster — keep going in this direction
            self.current_concurrency = (self.current_concurrency as i8 + self.direction) as usize;
            self.current_concurrency = self.current_concurrency.clamp(1, 32);
            self.stable_count = 0;
        } else if current_throughput < self.last_throughput * 0.95 {
            // Going slower — reverse direction, back off
            self.direction = -self.direction;
            self.current_concurrency = (self.current_concurrency as i8 + self.direction) as usize;
            self.current_concurrency = self.current_concurrency.clamp(1, 32);
            self.stable_count = 0;
        } else {
            // Stable — we found the sweet spot
            self.stable_count += 1;
        }

        self.last_throughput = current_throughput;
        self.current_concurrency
    }
}
```

## Device Detection (macOS)

### Drive Mount/Unmount
Using DiskArbitration framework:

```rust
// Pseudocode — actual implementation wraps DA* C APIs
fn watch_drives(db: SurrealDb) {
    let session = DASessionCreate(kCFAllocatorDefault);

    DARegisterDiskAppearedCallback(session, |disk| {
        let uuid = disk.volume_uuid();
        let mount = disk.mount_point();

        // Update or create drive record
        db.query("UPDATE drive SET connected = true, mount_point = $mount, last_seen = time::now() WHERE uuid = $uuid")
            .bind(("uuid", uuid))
            .bind(("mount", mount))
            .await;

        // Resume any intents waiting for this drive
        db.query("UPDATE intent SET status = 'scanning' WHERE status = 'waiting_for_device' AND (source.drive.uuid = $uuid OR destinations[*].drive.uuid CONTAINS $uuid)")
            .bind(("uuid", uuid))
            .await;
    });

    DARegisterDiskDisappearedCallback(session, |disk| {
        let uuid = disk.volume_uuid();

        db.query("UPDATE drive SET connected = false, mount_point = NONE WHERE uuid = $uuid")
            .bind(("uuid", uuid))
            .await;

        // Pause any active intents targeting this drive
        // (TransferJobs will detect write failures and trigger waiting_for_device)
    });
}
```

### Remote Machine Availability
Simple approach: periodic TCP connect to SSH port (or custom health endpoint).

```rust
async fn check_machine_online(machine: &Machine) -> bool {
    timeout(Duration::from_secs(5),
        TcpStream::connect(&machine.hostname)
    ).await.is_ok()
}
```

Run every 60s for known machines. When a machine transitions online → check for waiting intents.

## Data Directory

```
~/Library/Application Support/Kip/
├── kip.db/          # SurrealDB on-disk database (SurrealKV directory)
├── kip.log          # Application log (tracing-appender, non-rotating)
└── config.toml      # User preferences (future)
```

## Crate Dependencies (actual)

```toml
[dependencies]
dioxus = { version = "0.7.3", features = ["desktop", "router"] }
surrealdb = { version = "=3.0.0-beta.3", features = ["kv-surrealkv"] }
surrealdb-types = "=3.0.0-beta.4"  # needed for #[derive(SurrealValue)]
blake3 = "1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
walkdir = "2"
thiserror = "2"
plist = "1"                     # parsing diskutil output
tracing = "0.1"
tracing-subscriber = "0.3"
tracing-appender = "0.2"

# File preview (phase 2+)
# image = "0.25"
# syntect = "5"
# wgpu = "24"
# gltf = "1"
```

**Note**: SurrealDB 3.0.0-beta.4 has a broken dependency (`affinitypool ^0.4.0` not published) — pinned to beta.3. `surrealdb-types` must be a direct dependency because the `SurrealValue` derive macro references it by crate name.
