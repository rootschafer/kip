# Kip Data Model (SurrealDB)

## Why SurrealDB

SurrealDB isn't just "a database we're using" — it's architecturally load-bearing:

1. **Embedded mode**: Runs in-process via `surrealdb::engine::local::Db`. No separate server to manage. Kip is one binary.
2. **Graph relations**: `RELATE file -> exists_at -> location` — perfect for "where does this file live across all my machines?"
3. **LIVE SELECT**: Real-time subscriptions. The GUI subscribes to `LIVE SELECT * FROM transfer_job WHERE status = 'transferring'` and gets pushed updates without polling. The daemon writes, the GUI reacts. Zero coupling.
4. **Multi-model**: Documents for flexible metadata, graphs for relationships, traditional queries for reporting.

## Entities

### Machine
A computer or server Kip knows about.

```surql
DEFINE TABLE machine SCHEMAFULL;
DEFINE FIELD name ON machine TYPE string;            -- "MacBook", "derver"
DEFINE FIELD kind ON machine TYPE string;            -- "local", "remote"
DEFINE FIELD hostname ON machine TYPE option<string>; -- for remote: "ssh.anders.place"
DEFINE FIELD is_current ON machine TYPE bool;         -- is this the machine Kip is running on?
DEFINE FIELD ssh_user ON machine TYPE option<string>;
DEFINE FIELD ssh_key_path ON machine TYPE option<string>;
DEFINE FIELD ssh_proxy ON machine TYPE option<string>; -- e.g. cloudflared proxy command
DEFINE FIELD last_seen ON machine TYPE datetime;
DEFINE FIELD online ON machine TYPE bool DEFAULT false;
```

### Drive
A removable or mounted storage device.

```surql
DEFINE TABLE drive SCHEMAFULL;
DEFINE FIELD name ON drive TYPE string;              -- "SOMETHING", "DersThumb"
DEFINE FIELD uuid ON drive TYPE string;              -- filesystem UUID (survives rename)
DEFINE FIELD filesystem ON drive TYPE option<string>; -- "FAT32", "APFS", "ext4"
DEFINE FIELD capacity_bytes ON drive TYPE option<int>;
DEFINE FIELD mount_point ON drive TYPE option<string>; -- current mount path, null if disconnected
DEFINE FIELD connected ON drive TYPE bool DEFAULT false;
DEFINE FIELD last_seen ON drive TYPE datetime;
DEFINE FIELD limitations ON drive TYPE option<object>; -- { max_file_size: 4294967295 } for FAT32
```

**Why UUID?** The user might rename a drive or mount it at a different path. UUID is stable. When a drive mounts, Kip matches it by UUID, updates the mount_point, sets `connected = true`, and resumes any waiting intents.

### Location
The fundamental building block. Always: *something* + *path*.

```surql
DEFINE TABLE location SCHEMAFULL;
DEFINE FIELD machine ON location TYPE option<record<machine>>;  -- one of these
DEFINE FIELD drive ON location TYPE option<record<drive>>;      -- must be set
DEFINE FIELD path ON location TYPE string;                      -- "/Users/anders/projects"
DEFINE FIELD label ON location TYPE option<string>;             -- user-friendly: "My Projects"
DEFINE FIELD created_at ON location TYPE datetime;

-- A location is available if its machine is online or its drive is connected
DEFINE FIELD available ON location TYPE bool DEFAULT false;
```

A location always resolves to a concrete filesystem path:
- Machine location: `machine.hostname + path` (or just `path` if local)
- Drive location: `drive.mount_point + path`

### Intent
The core concept. "I want files from here to end up there."

```surql
DEFINE TABLE intent SCHEMAFULL;
DEFINE FIELD name ON intent TYPE option<string>;     -- user label: "Backup projects"
DEFINE FIELD source ON intent TYPE record<location>;
DEFINE FIELD destinations ON intent TYPE array<record<location>>;
DEFINE FIELD status ON intent TYPE string;           -- see lifecycle doc
DEFINE FIELD kind ON intent TYPE string;             -- "one_shot" | "sync"
DEFINE FIELD speed_mode ON intent TYPE string;       -- "normal" | "ninja" | "blast"
DEFINE FIELD priority ON intent TYPE int DEFAULT 0;  -- higher = processed first
DEFINE FIELD created_at ON intent TYPE datetime;
DEFINE FIELD updated_at ON intent TYPE datetime;

-- Progress tracking (aggregated from jobs)
DEFINE FIELD total_files ON intent TYPE int DEFAULT 0;
DEFINE FIELD total_bytes ON intent TYPE int DEFAULT 0;
DEFINE FIELD completed_files ON intent TYPE int DEFAULT 0;
DEFINE FIELD completed_bytes ON intent TYPE int DEFAULT 0;

-- Filters (what to include/exclude)
DEFINE FIELD include_patterns ON intent TYPE option<array<string>>; -- glob patterns
DEFINE FIELD exclude_patterns ON intent TYPE option<array<string>>; -- glob patterns
```

### TransferJob
A concrete unit of work: one file, one destination.

```surql
DEFINE TABLE transfer_job SCHEMAFULL;
DEFINE FIELD intent ON transfer_job TYPE record<intent>;
DEFINE FIELD source_path ON transfer_job TYPE string;       -- full resolved source path
DEFINE FIELD dest_path ON transfer_job TYPE string;         -- full resolved dest path
DEFINE FIELD destination ON transfer_job TYPE record<location>;
DEFINE FIELD size ON transfer_job TYPE int;                  -- file size in bytes
DEFINE FIELD bytes_transferred ON transfer_job TYPE int DEFAULT 0;
DEFINE FIELD status ON transfer_job TYPE string;
    -- "pending" | "transferring" | "verifying" | "complete" | "failed" | "needs_review"
DEFINE FIELD attempts ON transfer_job TYPE int DEFAULT 0;
DEFINE FIELD max_attempts ON transfer_job TYPE int DEFAULT 3;
DEFINE FIELD last_error ON transfer_job TYPE option<string>;
DEFINE FIELD error_kind ON transfer_job TYPE option<string>;
DEFINE FIELD source_hash ON transfer_job TYPE option<string>;  -- blake3
DEFINE FIELD dest_hash ON transfer_job TYPE option<string>;    -- blake3, after verify
DEFINE FIELD started_at ON transfer_job TYPE option<datetime>;
DEFINE FIELD completed_at ON transfer_job TYPE option<datetime>;
DEFINE FIELD created_at ON transfer_job TYPE datetime;
```

### FileRecord
Every file Kip has ever touched. The basis for dedup and change detection.

```surql
DEFINE TABLE file_record SCHEMAFULL;
DEFINE FIELD hash ON file_record TYPE string;           -- blake3 content hash
DEFINE FIELD size ON file_record TYPE int;
DEFINE FIELD first_seen ON file_record TYPE datetime;

-- Index for fast duplicate lookup
DEFINE INDEX idx_hash ON file_record FIELDS hash;
DEFINE INDEX idx_size ON file_record FIELDS size;
```

### FileRecord ↔ Location relationship (graph edge)
This is where SurrealDB's graph model shines.

```surql
DEFINE TABLE exists_at SCHEMAFULL;
DEFINE FIELD path ON exists_at TYPE string;              -- path within the location
DEFINE FIELD modified_at ON exists_at TYPE datetime;     -- file's mtime
DEFINE FIELD verified_at ON exists_at TYPE datetime;     -- when Kip last confirmed this
DEFINE FIELD stale ON exists_at TYPE bool DEFAULT false;  -- true if unverified after device reconnect

-- Usage: RELATE file_record:abc -> exists_at -> location:xyz
-- Query: "Where does this file exist?"
--   SELECT ->exists_at->location FROM file_record:abc
-- Query: "What files are at this location?"
--   SELECT <-exists_at<-file_record FROM location:xyz
-- Query: "Find duplicates" (same hash, different locations)
--   SELECT hash, count(), ->exists_at->location FROM file_record GROUP BY hash HAVING count > 1
```

### ReviewItem
An error that needs human attention.

```surql
DEFINE TABLE review_item SCHEMAFULL;
DEFINE FIELD job ON review_item TYPE record<transfer_job>;
DEFINE FIELD intent ON review_item TYPE record<intent>;
DEFINE FIELD error_kind ON review_item TYPE string;
    -- "conflict" | "permission_denied" | "disk_full" | "file_too_large"
    -- | "name_invalid" | "source_missing" | "hash_mismatch"
DEFINE FIELD error_message ON review_item TYPE string;
DEFINE FIELD source_path ON review_item TYPE string;
DEFINE FIELD dest_path ON review_item TYPE string;
DEFINE FIELD options ON review_item TYPE array<string>;   -- available resolutions
DEFINE FIELD resolution ON review_item TYPE option<string>; -- chosen resolution
DEFINE FIELD created_at ON review_item TYPE datetime;
DEFINE FIELD resolved_at ON review_item TYPE option<datetime>;

-- Source and dest file info for preview/comparison
DEFINE FIELD source_size ON review_item TYPE option<int>;
DEFINE FIELD source_hash ON review_item TYPE option<string>;
DEFINE FIELD source_modified ON review_item TYPE option<datetime>;
DEFINE FIELD dest_size ON review_item TYPE option<int>;
DEFINE FIELD dest_hash ON review_item TYPE option<string>;
DEFINE FIELD dest_modified ON review_item TYPE option<datetime>;
```

## Self-Maintaining Index

The file index must stay accurate without expiration-based deletion. Here's how:

1. **On transfer**: Hash file during copy. Create/update `file_record` and `exists_at` edge.
2. **On device connect**: Mark all `exists_at` edges for that device's locations as `stale = true`. As Kip encounters each file (during scan or transfer), verify and mark `stale = false`.
3. **On file watch event** (local/mounted): Update `exists_at` edge immediately. If file deleted, remove the edge. If `file_record` has zero remaining edges, it's an orphan — archive or delete it.
4. **On intent scan**: Verify files at source location, update records.
5. **Orphan cleanup**: Periodically (or on demand), find `file_record` nodes with no `exists_at` edges → delete them.

No timers. No TTLs. Records exist as long as Kip has evidence the file exists somewhere. Evidence disappears → record disappears.

## Key Queries

```surql
-- All files unique to one location (would be lost if that drive dies)
SELECT * FROM file_record WHERE count(->exists_at) = 1;

-- All duplicates (same content in multiple places)
SELECT hash, array::group(->exists_at->location) as locations
FROM file_record
GROUP BY hash
HAVING count(->exists_at) > 1;

-- What changed since last sync?
SELECT * FROM exists_at
WHERE out = $location
AND modified_at > $last_sync;

-- How much space is a drive using for duplicates?
SELECT math::sum(size) as wasted_bytes
FROM file_record
WHERE ->exists_at CONTAINS $drive_location
AND count(->exists_at) > 1;
```
