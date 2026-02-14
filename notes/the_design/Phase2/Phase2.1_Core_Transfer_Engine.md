# Core Transfer Engine Implementation

## Parent Task: Phase 2.1 Core Transfer Engine

This document details the implementation of the actual file transfer functionality in Kip.

## Overview

The transfer engine is responsible for moving files between locations based on user-defined intents. It handles chunked transfers, progress tracking, error recovery, and integrity verification.

## Core Components

### Transfer Job Model
- Represents a single file transfer operation
- Contains source/destination paths, progress, status
- Tracks bytes transferred, errors, and completion state

### Chunked File Transfer
- Files transferred in configurable chunks (default 256KB)
- Progress reported after each chunk
- Resume capability for interrupted transfers
- Integrity verification using Blake3 hashing

### Transfer Scheduler
- Manages concurrent transfer jobs
- Respects system resources and user-defined limits
- Handles retries for transient errors
- Prioritizes transfers based on user settings

## Data Model

### Database Schema
```
DEFINE TABLE transfer_job SCHEMAFULL;
DEFINE FIELD intent ON transfer_job TYPE record<intent>;
DEFINE FIELD source_path ON transfer_job TYPE string;
DEFINE FIELD dest_path ON transfer_job TYPE string;
DEFINE FIELD status ON transfer_job TYPE string VALUE $value OR 'pending';
DEFINE FIELD total_bytes ON transfer_job TYPE int;
DEFINE FIELD transferred_bytes ON transfer_job TYPE int DEFAULT 0;
DEFINE FIELD error ON transfer_job TYPE option<string>;
DEFINE FIELD created_at ON transfer_job TYPE datetime;
DEFINE FIELD updated_at ON transfer_job TYPE datetime;

DEFINE EVENT update_timestamp ON transfer_job WHEN $event == 'UPDATE' THEN ( $value = time::now() );
```

### Transfer Job Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferJob {
    pub id: RecordId,
    pub intent: RecordId,  // Associated intent
    pub source_path: String,
    pub dest_path: String,
    pub status: String,  // "pending", "transferring", "complete", "failed", "paused"
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

## Implementation Details

### File Scanning
1. Compare source and destination file metadata
2. Identify files that need transfer (missing, newer, different size/hash)
3. Generate transfer job list
4. Handle conflicts (different content, permissions, etc.)

### Chunked Transfer Process
1. Open source and destination files
2. Read chunk from source
3. Write chunk to destination
4. Update progress in database
5. Verify integrity if requested
6. Handle errors and retries

### Integrity Verification
- Blake3 hashing for content verification
- Single-pass hash calculation during transfer
- Comparison with expected hash
- Automatic retry on hash mismatch

### Resume Capability
- Track progress in database
- Identify partially transferred files
- Resume from last known position
- Handle corrupted partial transfers

## Performance Considerations

### Concurrency Control
- Configurable concurrent transfer limits
- Adaptive rate limiting based on system load
- Throttling for "Ninja" and "Blast" modes

### Memory Management
- Streaming transfers to minimize memory usage
- Buffer management for chunked operations
- Cleanup of temporary files and metadata

## Error Handling

### Retryable Errors
- Network timeouts
- Temporary permission issues
- Disk space temporarily unavailable

### Fatal Errors
- File deleted during transfer
- Permission denied permanently
- Destination disk full

## Integration Points

### With Intent System
- Creates jobs based on intent definitions
- Updates intent status based on job progress
- Handles bidirectional sync requirements

### With UI
- Reports progress through database
- Updates node status indicators
- Triggers error notifications

## Success Criteria

- [ ] File transfer jobs created from intents
- [ ] Chunked transfer with progress tracking
- [ ] Integrity verification with Blake3
- [ ] Resume capability for interrupted transfers
- [ ] Proper error handling and retry logic
- [ ] Performance acceptable for large files
- [ ] Integration with UI status indicators
- [ ] Resource management (concurrency, memory)