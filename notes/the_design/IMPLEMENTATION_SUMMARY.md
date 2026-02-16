# Kip Implementation Summary

**Last Updated:** Current Session
**Status:** Core infrastructure complete, force-directed graph needs implementation

---

## Overview

Kip is a file synchronization orchestrator with a force-directed graph UI. Users can visualize file locations across devices and create sync relationships by drawing edges between nodes.

---

## Current Implementation Status

### ✅ Completed Components

#### 1. Database Layer (`src/db.rs`)
- SurrealDB connection management
- Schema initialization (machine, drive, location, intent, review_item tables)
- Drive watcher for detecting mounted drives
- Query helpers for loading graph data

#### 2. App Structure (`src/app.rs`)
- Root component with global state
- Signal-based state management (`refresh_tick`, `hostname`)
- Proper use of `use_effect` for async operations (no infinite loops)
- Context providers for database access

#### 3. Graph Component (`src/ui/graph.rs`)
- `MappingGraph` component as main graph view
- `GraphToolbar` with machine chips and status indicator
- Resource-based data loading with proper signal handling
- Add machine panel with form

#### 4. Graph State (`src/ui/graph_store.rs`)
- `Graph` struct with nodes, edges, containers
- Signal-based state management
- Node position saving to database
- Drag state tracking (lasso, edge creation, node dragging)
- Physics simulation constants (not yet fully implemented)

#### 5. Graph Rendering
- **Nodes** (`src/ui/graph_nodes.rs`): `GraphNodeComponent` with proper event handling
- **Edges** (`src/ui/graph_edges.rs`): `GraphSvgOverlay` for SVG edge rendering
- **Types** (`src/ui/graph_types.rs`): `GraphNode`, `GraphEdge`, `NodeKind`, `Vec2`

#### 6. File Picker (`src/ui/file_picker.rs`)
- Custom column-based file navigation
- Multi-pane support (browse multiple locations simultaneously)
- Drag-to-workspace capability
- Persistent state (minimizes instead of closing)

#### 7. Notification System (`src/ui/notification.rs`)
- Toast notifications with levels (info, warning, error, progress)
- Spinner for ongoing operations
- Auto-dismiss with configurable timeout
- Progress bar support

#### 8. Review Queue (`src/ui/review_queue.rs`)
- Conflict resolution UI
- Review item display with metadata
- Resolution actions (skip, overwrite, merge)

---

### ❌ Incomplete / Not Working

#### 1. Force-Directed Layout
**Status:** Partially implemented, not functional
**Location:** `src/ui/graph_store.rs`, `src/ui/graph_nodes.rs`

**What's Missing:**
- Physics simulation loop (currently disabled to prevent infinite loops)
- Node positioning based on forces (repulsion, attraction, collision)
- Animation for node expansion/collapse
- Orbit view for directory children

**Reference:** See `external/nexus-node-sync/components/GraphCanvas.tsx` for D3 implementation

#### 2. Directory Expansion
**Status:** Data structure ready, UI not implemented
**Location:** `src/ui/graph_store.rs` - `toggle_expand()`, `wake()`

**What's Missing:**
- Visual expansion animation
- Orbit positioning for children
- "Enter directory" view (filtering workspace to show only children)
- Breadcrumb navigation for nested directories

**Reference:** See `external/nexus-node-sync/App.tsx` - `handleNodeClick()` expansion logic

#### 3. Node Grouping
**Status:** Not implemented
**Location:** N/A

**What's Missing:**
- Group creation from selected nodes
- Group node rendering (circle containing other nodes)
- Group expansion/collapse
- Edge merging for grouped nodes

**Reference:** See `external/nexus-node-sync/App.tsx` - `handleGroupNodes()`

#### 4. Edge Creation
**Status:** Drag state tracking exists, creation incomplete
**Location:** `src/ui/graph.rs` - mouse event handlers

**What's Missing:**
- Visual edge preview during drag
- Edge completion on node drop
- Intent creation in database
- Edge status visualization (idle, syncing, complete, error)

**Reference:** See `external/nexus-node-sync/App.tsx` - `linkMode` and `handleNodeClick()`

#### 5. Lasso Selection
**Status:** Drag state tracking exists, selection incomplete
**Location:** `src/ui/graph_store.rs` - `select_in_rect()`

**What's Missing:**
- Visual selection rectangle during drag
- Multi-node drag (move selected nodes together)
- Keyboard shortcuts (Shift+click, Ctrl+click)

**Reference:** See `external/nexus-node-sync/components/GraphCanvas.tsx` - `dragBehavior`

#### 6. Node Visual Design
**Status:** Basic rendering exists, styling incomplete
**Location:** `src/ui/graph_nodes.rs`

**What's Missing:**
- Circle nodes for directories/groups (currently all same shape)
- Size based on descendant count
- Status indicators (synced, syncing, error, offline)
- Selection glow/highlight
- Gradient fills (see TypeScript reference for colors)

---

## Architecture

### State Management

```
┌─────────────────────────────────────────────────────────┐
│                     App Component                       │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ refresh_tick│  │   picker     │  │   notifs      │  │
│  │  (Signal)   │  │  (Store)     │  │   (Store)     │  │
│  └─────────────┘  └──────────────┘  └───────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   MappingGraph                          │
│  ┌─────────────┐  ┌──────────────┐                     │
│  │    graph    │  │ loaded_data  │                     │
│  │  (Signal)   │  │  (Resource)  │                     │
│  └─────────────┘  └──────────────┘                     │
│         │                                              │
│         ▼                                              │
│  ┌─────────────────────────────────────────────┐      │
│  │              Graph (struct)                  │      │
│  │  - nodes: Vec<GraphNode>                    │      │
│  │  - edges: Vec<GraphEdge>                    │      │
│  │  - drag_state: DragState                    │      │
│  │  - selected: HashSet<String>                │      │
│  │  - sim_running: bool                        │      │
│  └─────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────┘
```

### Data Flow

```
User Action → EventHandler → Signal Update → use_effect → Side Effect
                                                          │
                                                          ▼
                                                  Database / Resource
                                                          │
                                                          ▼
                                                  Signal Update → UI Re-render
```

### Component Hierarchy

```
App
├── MappingGraph
│   ├── GraphToolbar
│   │   └── MachineChip (×N)
│   ├── Workspace (div)
│   │   ├── GraphSvgOverlay (SVG)
│   │   │   ├── Edge paths (×N)
│   │   │   └── Lasso rectangle
│   │   └── GraphNodeComponent (×N)
│   │       ├── DirectoryNode (circle)
│   │       ├── FileNode (pill)
│   │       └── DeviceNode (circle)
│   └── AddMachinePanel
├── FilePickerLayer
│   └── PickerPane (×N)
│       └── ColumnView (×N)
├── ReviewQueue
└── NotificationLayer
    └── NotificationToast (×N)
```

---

## Key Files

### Core
- `src/main.rs` - Entry point, logging setup (console only, WARN level)
- `src/app.rs` - Root component, global state
- `src/db.rs` - Database initialization, queries

### UI Components
- `src/ui/mod.rs` - Module exports
- `src/ui/graph.rs` - Main graph component
- `src/ui/graph_store.rs` - Graph state management
- `src/ui/graph_nodes.rs` - Node rendering
- `src/ui/graph_edges.rs` - Edge rendering
- `src/ui/graph_types.rs` - Type definitions
- `src/ui/file_picker.rs` - File picker component
- `src/ui/notification.rs` - Notification system
- `src/ui/container_components.rs` - Machine chips, add panel

### Types
- `src/models/mod.rs` - Database models
- `src/devices/macos.rs` - macOS-specific device detection

---

## Known Issues & Gotchas

### 1. Infinite Loops (CRITICAL)

**Symptom:** App freezes, CPU spikes, log file grows to 100+ GB

**Causes:**
- `spawn()` in component body without `use_effect()`
- Resource updating signal that triggers resource again
- Signal captured by closure instead of value

**Prevention:**
```rust
// Always wrap spawns in use_effect
use_effect(move || {
    spawn(async move { /* ... */ });
});

// Don't update signals from inside use_resource
// Instead, use separate use_effect to watch resource
let data = use_resource(move || async move { load().await });
use_effect(move || {
    if let Some(d) = data.read().as_ref() {
        signal.write().update(d);
    }
});
```

### 2. Signal vs Value Capture

```rust
// ❌ Wrong - captures Signal<u32>
use_resource(move || {
    let tick = refresh_tick;
    async move { /* ... */ }
});

// ✅ Correct - captures u32 value
use_resource(move || {
    let tick = refresh_tick; // tick is u32
    async move {
        let _ = tick; // Use value
        /* ... */
    }
});
```

### 3. DbHandle Moves

```rust
// Clone before moving into multiple closures
let db_for_resource = db.clone();
let loaded_data = use_resource(move || {
    let db_val = db_for_resource.clone();
    async move { /* ... */ }
});

// db is still available here for other uses
let db_for_handler = db.clone();
rsx! {
    button {
        onclick: move |_| {
            let db = db_for_handler.clone();
            spawn(async move { /* ... */ });
        }
    }
}
```

### 4. Logging

```rust
// ❌ Don't enable file logging (will fill disk)
// ✅ Console only, WARN level
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::WARN)
    .init();
```

---

## Reference Implementation Notes

### TypeScript Force-Directed Graph (`external/nexus-node-sync/`)

**D3 Configuration:**
```typescript
d3.forceSimulation(nodes)
  .force("link", d3.forceLink(links)
    .id(d => d.id)
    .distance(d => d.type === 'sync' ? 150 : 80))
  .force("charge", d3.forceManyBody().strength(-300))
  .force("x", d3.forceX().strength(0.04))
  .force("y", d3.forceY().strength(0.04))
  .force("collide", d3.forceCollide()
    .radius(d => d.type === 'device' ? 45 : 30)
    .iterations(2))
```

**Node Types:**
- `root` - Purple gradient, largest
- `device` - Blue gradient, large circle
- `folder` - Slate gradient, medium circle
- `group` - Green gradient, medium circle
- `file` - Small circle (or pill shape)

**Interactions:**
- Click node → Select / Expand
- Shift+click → Multi-select
- Shift+drag background → Lasso select
- Drag node → Move (with physics)
- Link mode → Create sync edge

**Key Patterns to Port:**
1. Simulation restart on node add/remove
2. Drag fixes position temporarily (`fx`, `fy`)
3. Multi-node drag (drag selection together)
4. Zoom/pan with mouse wheel
5. Selection glow effect

---

## Next Development Priorities

### Phase 1: Force-Directed Layout
1. Implement physics simulation (use existing Rust crate or port D3 logic)
2. Add simulation loop with proper start/stop control
3. Update node positions on tick
4. Add drag-to-move with physics

### Phase 2: Directory Expansion
1. Implement orbit positioning for children
2. Add expansion animation
3. Implement "enter directory" view
4. Add breadcrumb navigation

### Phase 3: Node Grouping
1. Create group from selected nodes
2. Render group as containing circle
3. Implement group expand/collapse
4. Merge edges for grouped nodes

### Phase 4: Polish
1. Node visual design (gradients, sizes, status indicators)
2. Edge status visualization
3. Smooth animations
4. Performance optimization for 100+ nodes

---

## Build & Run

```bash
# Build
dx build

# Run in dev mode
dx serve --platform desktop

# Check without building
dx check
```

**Note:** Always use `dx` commands, not `cargo build`.

---

## Testing Checklist

Before marking a feature complete:
- [ ] No infinite loops (monitor CPU and log file size)
- [ ] No console errors
- [ ] Works with 0 nodes (empty state)
- [ ] Works with 100+ nodes (performance)
- [ ] All interactions work (click, drag, lasso, etc.)
- [ ] State persists across app restart (if applicable)
- [ ] No memory leaks (monitor over time)
