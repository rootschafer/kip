# Kip CLI Design

**Date:** February 21, 2026  
**Status:** Design Document

---

## Overview

The Kip CLI (`kip-cli` binary) provides command-line access to all Kip functionality. It is a **thin wrapper** around the API layer — every command maps directly to `api::*` functions.

### Binary Structure

```
kip/
├── Cargo.toml          # Defines [[bin]] for kip-cli
├── src/
│   ├── main.rs         # GUI binary (Dioxus)
│   ├── bin/
│   │   └── kip-cli.rs  # CLI binary entry point
│   ├── lib.rs          # Library root (exports api::)
│   └── ...
```

### Cargo.toml Configuration

```toml
[[bin]]
name = "kip-cli"
path = "src/bin/kip-cli.rs"

[dependencies]
# Same as existing + clap
clap = { version = "4.4", features = ["derive"] }
```

---

## Command Hierarchy

```
kip
├── status              # Show overall system status
├── intent
│   ├── list            # List all intents
│   ├── create          # Create new intent
│   ├── show <ID>       # Show intent details
│   ├── delete <ID>     # Delete intent
│   ├── run <ID>        # Run transfer
│   └── cancel <ID>     # Cancel running intent
├── location
│   ├── list            # List all locations
│   ├── add <PATH>      # Add location
│   └── remove <ID>     # Remove location
├── review
│   ├── list            # List items needing review
│   ├── show <ID>       # Show review item details
│   ├── resolve <ID>    # Resolve single item
│   └── resolve-all     # Bulk resolve
├── config
│   ├── import          # Import backup-tool configs
│   └── export          # Export current config
└── run                 # Shorthand: run all idle intents
```

---

## Command Specifications

### `kip status`

Show overall system status.

```bash
kip status
```

**Output:**
```
Kip Status — 2026-02-21 14:30:00

Intents
  Total: 5    Idle: 2    Transferring: 1    Complete: 2    Needs Review: 0

Transfers
  Pending: 12    Transferring: 4    Complete: 156    Failed: 3

Review Queue
  Total: 0

Drives
  Connected:
    ✅ My Passport    /Volumes/My Passport    1.0 TB available
    ✅ ARCHIVE        /Volumes/ARCHIVE        3.2 TB available
  
  Disconnected:
    ❌ Time Capsule   (last seen: 2026-02-20)
```

**API Calls:**
- `api::status()`

---

### `kip intent list`

List all intents with summary.

```bash
kip intent list [--status STATUS]
```

**Options:**
- `--status <STATUS>` — Filter by status (`idle`, `transferring`, `complete`, `needs_review`)

**Output:**
```
ID                                    Source                      Destinations              Status         Progress
────────────────────────────────────────────────────────────────────────────────────────────────────────────────
intent:backup_obsidian                ~/Documents/Obsidian        My Passport:/backups/...  complete       100% (234 MB)
intent:sync_photos                    ~/Pictures                  ARCHIVE:/photos/...       transferring   45% (1.2 GB / 2.7 GB)
intent:dotfiles                       ~/.dotfiles                 My Passport:/dotfiles     idle           -
```

**API Calls:**
- `api::list_intents()`

---

### `kip intent create`

Create a new intent.

```bash
kip intent create <SOURCE> <DESTINATION>... [OPTIONS]
```

**Arguments:**
- `<SOURCE>` — Source path (e.g., `~/Documents/Obsidian`)
- `<DESTINATION>` — One or more destinations (e.g., `My Passport:/backups/obsidian`)

**Options:**
- `--name <NAME>` — Human-readable name
- `--priority <N>` — Priority 0-1000 (default: 500)
- `--speed <MODE>` — `fast`, `throttled`, or `background` (default: `fast`)
- `--include <PATTERN>` — Glob pattern to include (repeatable)
- `--exclude <PATTERN>` — Glob pattern to exclude (repeatable)
- `--bidirectional` — Enable bidirectional sync

**Examples:**
```bash
# Simple backup
kip intent create ~/Documents/Obsidian "My Passport:/backups/obsidian"

# With name and priority
kip intent create ~/.dotfiles "My Passport:/dotfiles" --name "Dotfiles Backup" --priority 800

# With exclusions
kip intent create ~/code/myproject "ARCHIVE:/code/myproject" \
  --exclude "**/target/**" \
  --exclude "**/node_modules/**"
```

**Output:**
```
Created intent: intent:backup_obsidian
  Source: ~/Documents/Obsidian
  Destinations:
    - My Passport:/backups/obsidian
  
Run with: kip intent run intent:backup_obsidian
```

**API Calls:**
1. Resolve source path → `api::add_location()` (if not exists)
2. Resolve each destination → `api::add_location()` (if not exists)
3. `api::create_intent(source_id, dest_ids, config)`

---

### `kip intent show <ID>`

Show detailed intent information.

```bash
kip intent show <INTENT_ID>
```

**Output:**
```
Intent: intent:backup_obsidian
  Name: Obsidian Vault Backup
  Status: complete
  Created: 2026-02-15 10:00:00
  Updated: 2026-02-21 09:30:00

  Source:
    Path: ~/Documents/Obsidian
    Machine: local

  Destinations:
    - My Passport:/backups/obsidian (connected)

  Configuration:
    Speed Mode: fast
    Priority: 500
    Include: *
    Exclude: **/.DS_Store
    Bidirectional: no

  Progress:
    Files: 234 / 234 (100%)
    Bytes: 234.5 MB / 234.5 MB

  Recent Transfers:
    ✅ 2026-02-21 09:30:00  234 files, 234.5 MB
    ✅ 2026-02-20 09:00:00  230 files, 230.1 MB
```

**API Calls:**
- `api::get_intent(intent_id)`
- `api::transfer_history(Some(intent_id), Some(10))`

---

### `kip intent delete <ID>`

Delete an intent.

```bash
kip intent delete <INTENT_ID> [--force]
```

**Options:**
- `--force` — Delete even if transfers are in progress

**Behavior:**
- Warns before deleting
- Does NOT delete transferred files
- Fails if intent is `transferring` (unless `--force`)

**API Calls:**
- `api::delete_intent(intent_id)`

---

### `kip intent run <ID>`

Run transfer for an intent.

```bash
kip intent run <INTENT_ID> [--verbose]
```

**Options:**
- `--verbose` — Show per-file progress

**Output (normal):**
```
Running intent: intent:backup_obsidian

Scanning source: ~/Documents/Obsidian
  Found: 234 files, 234.5 MB

Transferring...
  ████████████████████████░░░░  45%  105 MB / 234 MB

✅ Complete: 234 files, 234.5 MB in 12.3s
```

**Output (verbose):**
```
Running intent: intent:backup_obsidian

Scanning...
  Found: 234 files

Transferring:
  ✅ index.md                          1.2 KB
  ✅ Daily/2026-02-21.md               4.5 KB
  ⏳ Projects/kip.md                   8.9 KB  [████░░░░] 45%
  ⏳ Projects/architecture.md          12.3 KB [██░░░░░░] 22%
  ...

✅ Complete: 234 files, 234.5 MB
```

**API Calls:**
- `api::run_intent(intent_id, Some(progress_callback))`

---

### `kip intent cancel <ID>`

Cancel a running intent.

```bash
kip intent cancel <INTENT_ID>
```

**Behavior:**
- Sets intent status to `error`
- Cancels pending and transferring jobs
- Already-completed transfers remain

**API Calls:**
- `api::cancel_intent(intent_id)`

---

### `kip location list`

List all registered locations.

```bash
kip location list
```

**Output:**
```
ID                        Path                              Machine      Available
───────────────────────────────────────────────────────────────────────────────────
location:src_001          ~/Documents/Obsidian              local        ✅
location:dst_001          /Volumes/My Passport/backups/...  My Passport  ✅
location:dst_002          /Volumes/ARCHIVE/photos           ARCHIVE      ✅
location:dst_003          /Volumes/Time Capsule/backups     Time Capsule ❌
```

**API Calls:**
- `api::list_locations()`

---

### `kip location add <PATH>`

Add a new location.

```bash
kip location add <PATH> [--label LABEL] [--machine MACHINE]
```

**Arguments:**
- `<PATH>` — Absolute path or path with `~`

**Options:**
- `--label <LABEL>` — Human-readable label
- `--machine <MACHINE>` — Machine name (default: `local`)

**Examples:**
```bash
kip location add ~/Documents/Obsidian --label "Obsidian Vault"
kip location add /Volumes/My\ Passport/backups --machine "My Passport"
```

**API Calls:**
- `api::add_location(path, label, machine)`

---

### `kip location remove <ID>`

Remove a location.

```bash
kip location remove <LOCATION_ID>
```

**Behavior:**
- Fails if location is referenced by active intents

**API Calls:**
- `api::remove_location(location_id)`

---

### `kip review list`

List items needing review.

```bash
kip review list [--intent INTENT_ID]
```

**Options:**
- `--intent <ID>` — Filter by intent

**Output:**
```
ID                  Intent                        Error              Source → Dest
────────────────────────────────────────────────────────────────────────────────────────────────
review:001          intent:sync_photos            Source Not Found   ~/Pictures/old/IMG_001.jpg
                                                      → ARCHIVE:/photos/old/IMG_001.jpg
review:002          intent:sync_photos            Permission Denied  ~/Pictures/private/
                                                      → ARCHIVE:/photos/private/
```

**API Calls:**
- `api::list_review_items()`

---

### `kip review show <ID>`

Show detailed review item.

```bash
kip review show <REVIEW_ID>
```

**Output:**
```
Review Item: review:001

  Intent: intent:sync_photos
  Error: Source Not Found
  
  Source:
    Path: ~/Pictures/old/IMG_001.jpg
    Size: (unknown)
    Modified: (unknown)
  
  Destination:
    Path: ARCHIVE:/photos/old/IMG_001.jpg
    Exists: no
  
  Options:
    1. retry        — Try again (source may be temporarily unavailable)
    2. skip         — Skip this file, continue with others
    3. abort        — Cancel entire intent
  
  Resolve with: kip review resolve review:001 --option skip
```

**API Calls:**
- `api::list_review_items()` (filter by ID)

---

### `kip review resolve <ID>`

Resolve a review item.

```bash
kip review resolve <REVIEW_ID> --option <OPTION>
```

**Options:**
- `retry` — Reset job to pending
- `skip` — Skip this file
- `overwrite` — Force overwrite (for conflicts)
- `delete-source` — Delete source and mark complete
- `delete-dest` — Delete destination and retry
- `abort` — Cancel entire intent

**API Calls:**
- `api::resolve_review(review_id, resolution)`

---

### `kip review resolve-all`

Resolve all review items for an intent.

```bash
kip review resolve-all <INTENT_ID> --option <OPTION>
```

**Options:**
- `--option <OPTION>` — Resolution to apply to all

**Output:**
```
Resolved 5 items with 'skip'
  Skipped: 5
  Retried: 0
  Failed: 0
```

**API Calls:**
- `api::resolve_all_review(intent_id, resolution)`

---

### `kip config import`

Import backup-tool configuration.

```bash
kip config import [--config-dir PATH] [--dry-run]
```

**Options:**
- `--config-dir <PATH>` — Config directory (default: `~/.config/backup-tool`)
- `--dry-run` — Show what would be imported without making changes

**Output (dry-run):**
```
Import Preview:

  Config Files:
    ✅ ~/.config/backup-tool/drives.toml
    ✅ ~/.config/backup-tool/apps/obsidian.toml
    ✅ ~/.config/backup-tool/apps/dotfiles.toml

  Would Create:
    Locations: 6
      - ~/Documents/Obsidian
      - ~/My\ Passport:/backups/obsidian
      - ~/.dotfiles
      - ~/My\ Passport:/dotfiles
      ...
    
    Intents: 3
      - obsidian → My Passport
      - dotfiles → My Passport
      - photos → ARCHIVE

Run without --dry-run to import.
```

**Output (actual):**
```
Import Complete:
  Locations Created: 6
  Intents Created: 3
  Errors: 0

Run 'kip intent list' to see imported intents.
```

**API Calls:**
- `api::import_backup_tool_config(config_dir)`

---

### `kip config export`

Export current configuration.

```bash
kip config export --format <FORMAT> --output <DIR>
```

**Options:**
- `--format <FORMAT>` — `toml` (backup-tool compatible) or `json` (full fidelity)
- `--output <DIR>` — Output directory

**API Calls:**
- `api::export_config(format, output_dir)`

---

### `kip run`

Shorthand to run all idle intents.

```bash
kip run [--all]
```

**Options:**
- `--all` — Run all intents, not just idle (re-scan complete ones)

**Output:**
```
Running 2 idle intents...

[intent:backup_obsidian]
  Scanning... 234 files, 234.5 MB
  Transferring... ████████████████████████  100%
  ✅ Complete

[intent:dotfiles]
  Scanning... 45 files, 1.2 MB
  Transferring... ████████████████████████  100%
  ✅ Complete

All intents complete.
```

**API Calls:**
- `api::list_intents()` (filter idle)
- `api::run_intent()` for each

---

## Error Handling

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (invalid args, API error) |
| 2 | Intent needs review (some transfers failed) |
| 3 | Location unavailable (drive not connected) |

### Error Output Format

```
❌ Error: Source path does not exist: /Users/anders/NonExistent

Hint: Check that the path exists and is accessible.
```

For review items:
```
✅ Transfer complete with warnings

⚠️  3 items need review:
   - Source Not Found: ~/Pictures/old/IMG_001.jpg
   - Permission Denied: ~/Pictures/private/
   - Hash Mismatch: ~/Documents/important.doc

Run 'kip review list' to resolve.
```

---

## Implementation Notes

### Progress Callback Implementation

```rust
fn make_progress_cb(verbose: bool) -> ProgressCallback {
    Arc::new(move |update: ProgressUpdate| {
        match update.kind {
            ProgressKind::Scanning { files_found, bytes_scanned } => {
                eprintln!("  Scanning... {} files, {}", files_found, format_bytes(bytes_scanned));
            }
            ProgressKind::Transferring { ref file, bytes_transferred } => {
                if verbose {
                    eprintln!("  {} {}", file, format_bytes(bytes_transferred));
                }
            }
            ProgressKind::Complete { files_transferred, bytes_transferred } => {
                eprintln!("✅ Complete: {} files, {}", files_transferred, format_bytes(bytes_transferred));
            }
        }
    })
}
```

### ID Parsing Helper

```rust
fn parse_intent_id(s: &str) -> Result<RecordId, KipError> {
    // Accept: "intent:backup_obsidian" or just "backup_obsidian"
    let s = s.strip_prefix("intent:").unwrap_or(s);
    Ok(RecordId::new("intent", s))
}
```

---

## Future Commands (Not Yet Implemented)

- `kip schedule list` — List scheduled intents
- `kip schedule add <INTENT> <CRON>` — Schedule an intent
- `kip remote add <HOST>` — Add remote machine
- `kip sync <INTENT>` — Run bidirectional sync
