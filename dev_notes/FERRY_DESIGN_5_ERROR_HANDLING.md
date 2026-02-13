# Error Handling & Review Queue

## Philosophy

Most file transfer errors are boring and fixable. Ferry handles them silently. The few that actually need a human get a clear, previewable, actionable review card.

The user should never think about errors unless Ferry genuinely can't decide what to do.

## Error Classification

### Auto-Resolved (user never sees these)

| Error | Strategy | Limit |
|-------|----------|-------|
| Network timeout | Retry with exponential backoff | 3 attempts |
| Connection refused | Queue, retry when machine comes online | Until online |
| Temporary I/O error | Retry immediately | 3 attempts |
| Connection reset | Retry with backoff | 3 attempts |
| Drive disconnected | Transition to `waiting_for_device` | Until reconnected |
| SSH auth expired | Re-authenticate, retry | 2 attempts then review |

After exhausting retries, auto-resolved errors promote to review items.

### Needs Review (user decides)

| Error Kind | What happened | Resolution Options |
|------------|--------------|-------------------|
| `conflict` | File exists at destination with different content | **Keep Source** / **Keep Dest** / **Keep Both** (rename) / **Preview Both** / **Skip** |
| `permission_denied` | Can't read source or write to destination | **Retry** / **Skip** / **Cancel Intent** |
| `disk_full` | Destination has no free space | **Free Space** (show usage) / **Choose Different Dest** / **Skip** |
| `file_too_large` | File exceeds filesystem limit (FAT32: 4GB) | **Split** / **Skip** |
| `name_invalid` | Filename contains chars invalid on dest filesystem | **Auto-Rename** (suggest) / **Custom Name** / **Skip** |
| `source_missing` | Source file disappeared during transfer | **Skip** / **Rescan Source** |
| `hash_mismatch` | File copied but verification failed | **Retry Transfer** / **Skip** / **Accept Anyway** |
| `auth_failed` | SSH/remote auth won't work after retries | **Re-enter Credentials** / **Skip** / **Cancel Intent** |

## Review Item Anatomy

Each review item in the UI contains:

```
┌────────────────────────────────────────────────────┐
│ ⚠ CONFLICT                              2 min ago │
│                                                    │
│ model_v3.stl                                       │
│ MacBook:/projects/cad/ → SOMETHING:/backup/cad/    │
│                                                    │
│ ┌──────────────────┬──────────────────┐            │
│ │ Source           │ Destination      │            │
│ │ 2.4 MB           │ 1.8 MB           │            │
│ │ Modified today    │ Modified Jan 28  │            │
│ │ [Preview]         │ [Preview]        │            │
│ └──────────────────┴──────────────────┘            │
│                                                    │
│ [Keep Source] [Keep Dest] [Keep Both] [Skip]       │
└────────────────────────────────────────────────────┘
```

### Expandable Preview
Clicking [Preview] opens inline file preview (when supported):
- Images: rendered inline
- 3D models: interactive viewport
- Code/text: syntax highlighted diff
- Other: hex dump or "preview not available" with file metadata

### Batch Resolution
When multiple items have the same error kind:
- "Apply to all similar" checkbox
- "Keep Source for all conflicts" as a bulk action
- Smart grouping: conflicts from the same intent are grouped

## Conflict Detection

Before writing a file, Ferry checks the destination:

```rust
enum ConflictCheck {
    NoConflict,          // dest doesn't exist
    Identical,           // dest exists, same hash → skip transfer
    Modified,            // dest exists, different hash → review
    SourceNewer,         // dest exists, source mtime > dest mtime
    DestNewer,           // dest exists, dest mtime > source mtime
}

fn check_conflict(source: &FileInfo, dest_path: &Path, db: &SurrealDb) -> ConflictCheck {
    if !dest_path.exists() {
        return ConflictCheck::NoConflict;
    }

    let dest_hash = blake3_hash(dest_path);

    if source.hash == dest_hash {
        return ConflictCheck::Identical; // already there, skip
    }

    // Different content — this is a real conflict
    if source.modified > dest_modified {
        ConflictCheck::SourceNewer
    } else {
        ConflictCheck::DestNewer
    }
}
```

For `SourceNewer` in one-shot intents: auto-overwrite (the user clearly wants the source version).
For `DestNewer` or ambiguous: create review item.

The user can configure per-intent conflict policy:
- `ask` (default) — create review item
- `overwrite` — always keep source
- `skip` — always keep dest
- `keep_both` — auto-rename source copy

## Notification Strategy

| Event | Notification |
|-------|-------------|
| Transfer complete (no errors) | Badge clears. Toast if window not open: "✓ Backup complete" |
| Transfer complete (with skips) | Toast: "Backup complete, 3 files skipped" |
| New review items | Menu bar badge count. Toast: "2 items need review" |
| Device connected, intents resuming | Toast: "SOMETHING connected, resuming backup" |
| Device disconnected mid-transfer | Toast: "SOMETHING disconnected, 5 transfers paused" |
| Error auto-resolved | Nothing. Silent. |

Notifications respect macOS notification settings. The user can disable them in Ferry preferences.

## Error Recovery Across Restarts

On launch, Ferry queries:

```surql
-- Find all jobs that were mid-transfer when we last quit/crashed
SELECT * FROM transfer_job WHERE status = 'transferring';
```

These jobs are reset to `pending` with their `bytes_transferred` preserved (for network resume) or reset to 0 (for local — faster to restart than seek).

Review items persist across restarts. They sit in the queue until the user resolves them.
