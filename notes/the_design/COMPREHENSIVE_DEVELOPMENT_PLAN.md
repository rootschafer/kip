# Kip Development Plan: From Current State to Production Release

**Date:** February 22, 2026  
**Status:** Living Document

---

## Executive Summary

Kip is a file transfer orchestrator with a spatial graph interface. Users create sync relationships by connecting location nodes in a 2D workspace. The app handles the complexity of monitoring, transferring, and resolving conflicts automatically.

---

## Current State Assessment

### Architecture Overview

| Component | Technology | Status |
|-----------|------------|--------|
| **UI Framework** | Dioxus 0.7.3 | ✅ Production-ready |
| **Database** | SurrealDB 3.0.0 (stable) | ✅ Production-ready |
| **Storage Engine** | SurrealKV (embedded) | ✅ Working |
| **Backend API** | Rust library layer | ✅ Implemented |
| **CLI** | clap + API layer | ✅ Implemented |
| **Transfer Engine** | Custom chunked copier | ✅ Implemented |

### Implemented Features

#### Core Infrastructure
- [x] SurrealDB embedded database with schema
- [x] API layer abstraction (`src/api/`)
- [x] CLI binary with full command set
- [x] Transfer engine with chunked copying
- [x] Filesystem scanner with symlink handling
- [x] Job scheduler with bounded concurrency
- [x] Error classification and review queue

#### UI Components
- [x] Free workspace with absolutely positioned nodes
- [x] Machine/Drive chips in toolbar
- [x] Node rendering (files as pills, directories as circles)
- [x] Force-directed layout with cluster separation
- [x] Edge creation via drag
- [x] Lasso selection
- [x] Multi-select and multi-drag
- [x] File picker with column navigation
- [x] Remote machine addition
- [x] Status indicators and notifications
- [x] Glassmorphic UI design

#### Data Model
- [x] Machine, Drive, Location entities
- [x] Intent (sync relationship) tracking
- [x] TransferJob with status tracking
- [x] ReviewItem for conflict resolution
- [x] FileRecord for deduplication

### Known Limitations

#### Interaction Model (TO BE FIXED)
- [ ] Single click currently selects AND starts drag (conflicting)
- [ ] No context menu for node operations
- [ ] No keyboard shortcuts implemented
- [ ] Edge creation interferes with node selection

#### Visual Features
- [ ] Orbit view for directory children (partially working)
- [ ] Enter view (navigate into directory)
- [ ] Node grouping/collapsing
- [ ] Layout persistence across sessions
- [ ] Progress visualization on edges

#### Transfer Features
- [ ] Bidirectional sync
- [ ] Scheduled/sync-on-change intents
- [ ] Remote (SSH) transfer support
- [ ] Bandwidth throttling
- [ ] Conflict auto-resolution rules

---

## Development Roadmap

### Phase 1: Interaction Model (Current Priority)

**Goal:** Fix fundamental interaction conflicts and add context menus

#### 1.1 Click Behavior Refactor
- **Single click:** Select only (no drag start)
- **Click + drag:** Move node(s)
- **Double click:** Open context menu
- **Status:** Design complete, implementation pending

#### 1.2 Context Menu System
- Node-type-specific menus (Machine/Drive, Directory, File)
- Keyboard shortcut integration
- Extensible action framework
- **Status:** Design complete, implementation pending

#### 1.3 Keyboard Shortcuts
- Global shortcuts (ESC, DELETE, SPACE, ENTER)
- Navigation shortcuts (ENTER into, BACKSPACE parent)
- Selection shortcuts (CMD+A, Shift+Click)
- **Status:** Design complete, implementation pending

**Deliverables:**
- Consistent, non-conflicting interactions
- Context menus for all node types
- Full keyboard navigation
- Updated interaction documentation

---

### Phase 2: Visual Enhancements

#### 2.1 Directory Expansion
- Orbit view: children fan out around parent circle
- Enter view: navigate into directory context
- Breadcrumb navigation for returning to parent
- Smooth animations between states

#### 2.2 Node Grouping
- Select multiple nodes → group into collapsible container
- Group shows summary ("5 locations")
- Edges reroute to group container
- Nested groups supported
- Persisted to database

#### 2.3 Layout Persistence
- Save node positions to database
- Restore layout on app launch
- Auto-layout for new nodes
- Manual position locking

#### 2.4 Edge Visualization
- Progress bars on transferring edges
- Edge bundling for parallel connections
- Animated flow indicators
- Hover details (file count, last sync time)

**Deliverables:**
- Intuitive directory navigation
- Reduced visual clutter via grouping
- Layout survives restarts
- Clear transfer status at a glance

---

### Phase 3: Transfer Engine Enhancements

#### 3.1 Bidirectional Sync
- Detect changes on both source and destination
- Merge strategies (newest wins, manual resolve)
- Conflict detection and flagging
- Sync history and undo

#### 3.2 Scheduled Intents
- Cron-like scheduling (hourly, daily, weekly)
- Sync on file change (filesystem watching)
- Sync on drive connect
- Pause/resume schedules

#### 3.3 Remote Transfer (SSH/SFTP)
- SSH key management
- Connection pooling
- Delta transfer (rsync-like)
- Compression for slow links
- Progress over network

#### 3.4 Performance Features
- Bandwidth throttling
- Parallel file transfers (configurable)
- Skip unchanged files (hash + mtime)
- Resume interrupted transfers

**Deliverables:**
- Set-and-forget sync relationships
- Efficient remote transfers
- Configurable performance tuning

---

### Phase 4: Platform Expansion

#### 4.1 Web Frontend
- Dioxus web target
- Actix-web backend API
- Real-time sync status via WebSocket
- Same API as desktop

#### 4.2 Linux Support
- Inotify for filesystem watching
- Udisks2 for drive detection
- GTK file picker integration
- AppImage/Flatpak distribution

#### 4.3 Windows Support
- ReadDirectoryChangesW for watching
- WMI for drive detection
- Native file picker
- MSI installer

#### 4.4 Cloud Destinations
- S3-compatible storage
- Google Drive
- Dropbox
- Backblaze B2

**Deliverables:**
- Cross-platform availability
- Cloud backup options
- Web-based monitoring

---

### Phase 5: Advanced Features

#### 5.1 Versioning
- Snapshot-based versioning
- Point-in-time restore
- Diff between versions
- Configurable retention policies

#### 5.2 Encryption
- End-to-end encryption for transfers
- Encrypted storage at destination
- Key management
- Per-intent encryption settings

#### 5.3 Collaboration
- Shared intents between users
- Access control and permissions
- Activity feed
- Notifications (email, push)

#### 5.4 Analytics
- Transfer statistics dashboard
- Bandwidth usage over time
- Storage growth trends
- Sync health monitoring

**Deliverables:**
- Enterprise-ready features
- Compliance capabilities
- Team collaboration

---

## Technical Debt

### Immediate (Fix Before Phase 2)
1. **Interaction conflicts** — Click vs drag ambiguity
2. **No context menu** — Operations not discoverable
3. **No keyboard shortcuts** — Power user workflows blocked

### Short-term (Fix in Phase 2-3)
1. **SurrealDB type coercion** — RecordId vs String issues (partially fixed)
2. **Schema evolution** — Need migration system for schema changes
3. **Error messages** — Some errors are cryptic

### Long-term (Architectural)
1. **Plugin system** — For custom transfer protocols
2. **Scripting API** — Lua/JS automation
3. **Microservices split** — Separate transfer daemon from UI

---

## Success Metrics

### User Experience
- Time to create first sync: < 30 seconds
- Time to resolve conflict: < 10 seconds
- Keyboard shortcut adoption: > 40% of users
- Context menu discoverability: > 80% find key actions

### Technical
- App startup time: < 2 seconds
- Sync latency (file change to transfer start): < 5 seconds
- Memory usage: < 500MB for 1000 nodes
- Transfer throughput: Within 10% of rsync

### Business
- User retention (30-day): > 60%
- NPS score: > 40
- Support tickets per 1000 users: < 10/month

---

## Risk Assessment

### High Risk
1. **SurrealDB stability** — Embedded mode is relatively new
   - Mitigation: Regular backups, migration path to client-server mode
2. **Web performance** — Large graphs in browser
   - Mitigation: Virtualization, level-of-detail rendering

### Medium Risk
1. **Cross-platform filesystem quirks** — Different semantics on Windows/Linux/macOS
   - Mitigation: Extensive testing, abstraction layer
2. **SSH key management** — Security-sensitive, platform-specific
   - Mitigation: Use established libraries, security audit

### Low Risk
1. **UI framework changes** — Dioxus is stable but evolving
   - Mitigation: Abstraction layer, pinned versions

---

## Appendix: File Structure

```
kip/
├── src/
│   ├── main.rs              # Desktop app entry
│   ├── lib.rs               # Library root
│   ├── bin/
│   │   └── kip-cli.rs       # CLI binary
│   ├── api/                 # API layer
│   │   ├── mod.rs
│   │   ├── intent.rs
│   │   ├── location.rs
│   │   ├── review.rs
│   │   ├── query.rs
│   │   └── config.rs
│   ├── engine/              # Transfer engine
│   │   ├── mod.rs
│   │   ├── transfer.rs      # Chunked copier
│   │   ├── scanner.rs       # Filesystem scanner
│   │   └── scheduler.rs     # Job scheduler
│   ├── db/                  # Database layer
│   │   ├── mod.rs
│   │   ├── schema.rs
│   │   └── init.rs
│   └── ui/                  # Dioxus UI
│       ├── graph.rs
│       ├── graph_nodes.rs
│       ├── graph_edges.rs
│       ├── graph_store.rs
│       └── file_picker.rs
├── crates/
│   └── actix-dioxus-serve/  # Web serving (future)
├── tests/                   # Integration tests
└── notes/
    └── the_design/          # This documentation
```

---

## Document History

| Date | Change | Author |
|------|--------|--------|
| 2026-02-13 | Initial draft | AI |
| 2026-02-17 | Updated with Phase progress | AI |
| 2026-02-22 | Major revision: accurate current state, expanded long-term | AI |

