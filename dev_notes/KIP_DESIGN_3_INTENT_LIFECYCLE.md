# Intent Lifecycle

## State Machine

```
                    ┌──────────────────────────────────────────┐
                    │              User creates                │
                    └──────────────┬───────────────────────────┘
                                   ▼
                              ┌─────────┐
              ┌───────────────│  idle    │◄──────────────┐
              │               └────┬────┘               │
              │                    │ trigger*            │ user resumes
              │                    ▼                     │
              │              ┌──────────┐          ┌────┴────┐
              │              │ scanning │          │ paused  │
              │              └────┬─────┘          └─────────┘
              │                   │                     ▲
              │                   │ files enumerated    │ user pauses
              │                   ▼                     │
              │           ┌──────────────┐              │
              │           │ transferring ├──────────────┘
              │           └──┬───┬───┬───┘
              │              │   │   │
              │   ┌──────────┘   │   └──────────┐
              │   ▼              ▼               ▼
        ┌─────┴────────┐  ┌──────────┐  ┌──────────────┐
        │ waiting_for  │  │verifying │  │ needs_review  │
        │   _device    │  └────┬─────┘  └──────┬───────┘
        └──────┬───────┘       │               │
               │               ▼               │ user resolves all
               │          ┌──────────┐         │
               │          │ complete │         │
               │          └──────────┘         │
               │                               │
               └───────► scanning ◄────────────┘
                         (rescan on reconnect / after review)
```

## States

### `idle`
Intent exists but isn't doing anything. Starting state after creation. Also the resting state for sync intents between runs.

### `scanning`
Kip is enumerating the source location. Building a manifest of files (path, size, mtime). For each destination, computing the delta: what's new, what's changed, what's already there. This is where Kip reads the file index to avoid redundant hashing.

### `transferring`
Actively copying files. TransferJobs are being executed according to the speed mode. Progress is tracked per-job and aggregated on the intent.

### `verifying`
All transfers for this batch are done. Kip is spot-checking or fully verifying that destination files match source hashes. Verification strategy depends on context:
- Local/USB: verify all (fast)
- Network: verify a sample or verify during transfer (hash while streaming)

### `complete`
All files transferred and verified. For one-shot intents, this is the final state. For sync intents, it returns to `idle` and waits for the next trigger.

### `waiting_for_device`
Source or destination became unavailable mid-operation (drive ejected, machine went offline, network dropped). Kip freezes the intent's jobs in place and waits. When the device returns:
- Kip transitions back to `scanning` (not `transferring`) because files may have changed while the device was away.

### `needs_review`
One or more TransferJobs hit errors that require human decision. The intent pauses and the errors appear in the review queue. Once all review items for this intent are resolved, it transitions back to `transferring` (or `scanning` if the resolution changed what needs to happen).

### `paused`
User manually paused the intent. All active TransferJobs are suspended (not cancelled — they track their byte offset). Resume returns to `idle`, which can be re-triggered.

## Triggers

What causes an idle intent to start scanning?

| Trigger | Description |
|---------|-------------|
| **User clicks "Start"** | Manual trigger |
| **Device connected** | Drive mounts or machine comes online that matches a source/destination |
| **File change detected** | Filesystem watcher sees changes in a sync intent's source |
| **Schedule** | Future feature: cron-like scheduling |

## TransferJob States

Individual file transfers have their own simple lifecycle:

```
pending → transferring → verifying → complete
                │
                ├──→ failed (retryable) ──→ pending (auto-retry)
                │
                └──→ needs_review (human required)
```

### Retry logic
- On retryable failure: increment `attempts`, wait `2^attempts` seconds, return to `pending`
- After `max_attempts` (default 3): promote to `needs_review`
- Retryable errors: timeout, connection reset, temporary I/O error
- Non-retryable errors: permission denied, disk full, conflict → immediate `needs_review`

## Intent Kinds

### `one_shot`
"Copy these files there." Scans once, transfers, completes. Done.

### `sync`
"Keep these locations in sync." After completing, returns to `idle`. Watches source for changes (via filesystem events or periodic scan). When changes detected, triggers a new scan → transfer cycle.

Future consideration: conflict resolution for bidirectional sync. MVP is unidirectional only (source → destination).

## Concurrency Rules

- Multiple intents can be active simultaneously
- Jobs from higher-priority intents are scheduled first
- In Ninja mode: only 1-2 concurrent jobs total
- In Normal mode: moderate concurrency (4-8 jobs, auto-tuned)
- In Blast mode: hill-climbing concurrency, per-destination tuning
- Two intents targeting the same destination share the destination's concurrency pool (don't overwhelm one drive with jobs from 5 different intents)

## Progress Persistence

Every state transition is written to SurrealDB before the transition executes. This means:

1. Kip crashes during `transferring` → on restart, query incomplete jobs, resume
2. Machine reboots → same. SurrealDB file is on disk.
3. Drive pulled mid-transfer → `waiting_for_device`. On reconnect, rescan + resume.

TransferJobs track `bytes_transferred`. For large files over network, this enables resume-from-byte. For local copies, Kip may restart the individual file (fast enough that partial resume isn't worth the complexity).
