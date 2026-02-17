# Kip Implementation Summary

**Last Updated:** February 17, 2026
**Status:** Force-directed graph fully functional, interaction features need completion

---

## Overview

Kip is a file synchronization orchestrator with a force-directed graph UI. Users visualize file locations across devices and create sync relationships by drawing edges between nodes.

---

## Current Implementation Status

### ✅ COMPLETED - Force-Directed Graph System

#### Physics Simulation (`src/ui/graph_store.rs`)
- **Repulsion force** (forceManyBody equivalent): 2000.0 strength - nodes push apart aggressively
- **Link attraction** (forceLink equivalent): 0.03 spring constant - connected nodes pull together
- **Center gravity** (forceX/forceY equivalent): 0.003 strength - very weak pull to center
- **Collision resolution** (forceCollide equivalent): 3 iterations, 0.7 strength
- **Alpha decay**: 0.97 per tick, stops when < 0.001
- **Edge length variation**:
  - Sync edges: 250px
  - Hierarchy edges: 180px
  - Group edges: 120px
- **Collision radii by node type**:
  - Device/Machine: 45px
  - Directory/Group: 30px
  - File: 15px

#### Simulation Loop (`src/ui/graph.rs`)
- Runs at ~60fps when active (16ms tick)
- Sleeps 100ms when idle to reduce CPU
- Wrapped in `use_effect` to prevent infinite loops
- Continuous loop (never exits, waits for sim_running)

#### Infinite Canvas & Viewport
- **Pan**: Alt+drag for 1:1 viewport movement
- **Viewport state**: `viewport_x`, `viewport_y`, `viewport_scale` in Graph struct
- **Transform**: CSS transform on container div (translate + scale)
- **No boundaries**: Nodes can spread infinitely

#### Drag-to-Move
- **fx/fy fields** on GraphNode for fixed positions during drag
- **fix_node_position()** - Sets fx/fy on drag start
- **release_node_position()** - Clears fx/fy on drag end
- **Restart simulation** after release to let node settle

#### Filesystem Scanning
- **scan_directory()** - Scans actual filesystem, creates nodes in orbit pattern
- **Orbit positioning** - 300px radius around parent
- **Hierarchy edges** - Automatically created for parent-child
- **Trigger**: Alt+click on machine/drive node

#### Visual Enhancements
- **Cluster backgrounds** - 350px radius circles, 8% opacity, machine/drive colors
- **Edges behind nodes** - SVG z-index: 1, nodes render on top
- **Edge preview line** - Transforms mouse coords to graph space for accurate tracking

### ✅ COMPLETED - Infrastructure

1. **Database Layer** (`src/db.rs`)
   - SurrealDB embedded connection
   - Schema: machine, drive, location, intent, review_item
   - Drive watcher for mount detection
   - Query helpers for graph data

2. **App Structure** (`src/app.rs`)
   - Root component with global state
   - `refresh_tick` signal for data refreshes
   - `hostname` state
   - Proper `use_effect` patterns (no infinite loops)

3. **Graph Component** (`src/ui/graph.rs`)
   - `MappingGraph` component
   - `GraphToolbar` with machine chips
   - Resource-based data loading
   - Add machine panel

4. **Graph State** (`src/ui/graph_store.rs`)
   - `Graph` struct with nodes, edges, containers
   - Signal-based state management
   - Position persistence to database
   - Drag state tracking (lasso, edge, node drag, panning)

5. **Graph Rendering**
   - **Nodes** (`src/ui/graph_nodes.rs`): FileNode, DirNode, GroupNode, MachineNode, DriveNode
   - **Edges** (`src/ui/graph_edges.rs`): SVG overlay with bezier curves
   - **Types** (`src/ui/graph_types.rs`): GraphNode, GraphEdge, NodeKind, Vec2

6. **File Picker** (`src/ui/file_picker.rs`)
   - Column-based navigation
   - Multi-pane support
   - Drag-to-workspace
   - Persistent (minimizes, doesn't close)

7. **Notification System** (`src/ui/notification.rs`)
   - Toast notifications (info, warning, error, progress)
   - Auto-dismiss with timeout
   - Progress bar support

8. **Review Queue** (`src/ui/review_queue.rs`)
   - Conflict resolution UI
   - Resolution actions (skip, overwrite, merge)

---

### ❌ INCOMPLETE / NEEDS WORK

#### 1. Zoom Functionality
**Status:** NOT WORKING - Dioxus API incompatibility
**Location:** `src/ui/graph.rs` - wheel handler removed

**Problem:**
- `WheelData` API varies across Dioxus versions
- No consistent `delta_y()` or field access

**Fix Needed:**
- Find correct Dioxus wheel event API
- Or implement zoom buttons as fallback
- Reference: `GraphCanvas.tsx` uses D3 zoom behavior

---

#### 2. Directory Expansion (Click to Expand)
**Status:** PARTIAL - toggle_expand() exists, needs filesystem integration

**What Works:**
- `toggle_expand()` in Graph struct
- Sets `expanded: true` on node kind
- Finds children by parent_id matching

**What's Missing:**
- On-demand filesystem scanning when expanding directories (not just machines/drives)
- Current implementation only scans machines/drives
- Directory nodes from DB don't trigger scan

**Fix Needed:**
- Extend scan logic to Directory nodes
- Pass directory path to scan function
- Show "Scanning..." status during async scan

**Reference:** `App.tsx` - `handleNodeClick()` expansion logic

---

#### 3. Edge Creation (Drag to Create Sync)
**Status:** PARTIAL - UI exists, DB creation missing

**What Works:**
- Ctrl/Alt+click starts edge creation
- Rubber band line follows cursor
- DragState::CreatingEdge tracks state

**What's Missing:**
- Drop on target node doesn't complete edge
- No intent created in database
- No visual feedback on hover over target

**Fix Needed:**
- Add onmouseup handler in node components
- Call `create_edge_in_db()` on drop
- Add edge to graph state
- Show "Create sync?" confirmation

**Reference:** `App.tsx` - `linkMode` and edge creation

---

#### 4. Lasso Selection
**Status:** PARTIAL - Drag tracking exists, multi-drag missing

**What Works:**
- Shift+drag creates selection rectangle
- `select_in_rect()` selects nodes in area
- Visual rectangle during drag

**What's Missing:**
- Multi-node drag (move all selected together)
- Selected nodes don't move together
- No visual indication of multi-selection during drag

**Fix Needed:**
- Track all selected nodes during drag
- Apply same offset to all selected
- Release all on drag end

**Reference:** `GraphCanvas.tsx` - `dragBehavior` multi-node drag

---

#### 5. Node Visual Design
**Status:** BASIC - All nodes use same styling

**What's Missing:**
- Circle nodes for directories/groups (currently all rectangular)
- Size based on descendant count (logarithmic scale)
- Status indicators (synced, syncing, error, offline)
- Selection glow/highlight
- Gradient fills (see TypeScript reference)

**Reference Values:**
```rust
fn calculate_node_size(descendants: usize) -> f64 {
    let log_count = (1.0 + descendants as f64).ln();
    (80.0 + log_count * 15.0).clamp(60.0, 150.0)
}
```

**Reference:** TypeScript gradients in `GraphCanvas.tsx` defs

---

#### 6. Node Grouping
**Status:** NOT IMPLEMENTED

**What's Missing:**
- Group creation from selected nodes
- Group node rendering (circle containing nodes)
- Group expand/collapse
- Edge merging for grouped nodes

**Reference:** `App.tsx` - `handleGroupNodes()`

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
│  │  - viewport_x/y/scale: f64                  │      │
│  │  - scanning: Option<String>                 │      │
│  │  - scan_progress: String                    │      │
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
│   ├── Workspace (div, 100% width/height)
│   │   ├── Viewport Transform (div)
│   │   │   ├── GraphSvgOverlay (SVG)
│   │   │   │   ├── Cluster circles
│   │   │   │   ├── Edge paths (×N)
│   │   │   │   ├── Rubber band line
│   │   │   │   └── Lasso rectangle
│   │   │   └── GraphNodeComponent (×N)
│   │   │       ├── FileNode (pill)
│   │   │       ├── DirNode (circle)
│   │   │       ├── GroupNode (circle)
│   │   │       ├── MachineNode (circle)
│   │   │       └── DriveNode (circle)
│   │   └── (pan/zoom handlers)
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
- `src/main.rs` - Entry point, logging (console only, WARN level)
- `src/app.rs` - Root component, global state
- `src/db.rs` - Database initialization, queries

### UI Components
- `src/ui/mod.rs` - Module exports
- `src/ui/graph.rs` - Main graph component, simulation loop, mouse handlers
- `src/ui/graph_store.rs` - Graph struct, physics, filesystem scanning
- `src/ui/graph_nodes.rs` - Node rendering components
- `src/ui/graph_edges.rs` - SVG edge overlay, cluster backgrounds
- `src/ui/graph_types.rs` - GraphNode, GraphEdge, NodeKind, Vec2
- `src/ui/file_picker.rs` - File picker component
- `src/ui/notification.rs` - Notification system
- `src/ui/container_components.rs` - Machine chips, add panel

### Types
- `src/models/mod.rs` - Database models
- `src/devices/macos.rs` - macOS device detection

---

## Known Issues & Gotchas

### 1. Infinite Loops (CRITICAL)

**Symptom:** App freezes, CPU spikes, log file grows

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

// Separate resource and effect
let data = use_resource(move || async move { load().await });
use_effect(move || {
    if let Some(d) = data.read().as_ref() {
        signal.write().update(d);
    }
});

// Capture values, not signals
use_resource(move || {
    let tick = refresh_tick; // tick is u32 VALUE
    async move { /* ... */ }
});
```

### 2. DbHandle Moves

```rust
// Clone before moving into multiple closures
let db_for_resource = db.clone();
let loaded_data = use_resource(move || {
    let db_val = db_for_resource.clone();
    async move { /* ... */ }
});

// db still available for other uses
let db_for_handler = db.clone();
```

### 3. Viewport Transform

All mouse coordinates must be transformed:
```rust
// Screen space → Graph space
let graph_x = (mouse_x - viewport_x) / viewport_scale;
let graph_y = (mouse_y - viewport_y) / viewport_scale;
```

### 4. Force Parameters

Current values tuned for cluster separation:
- High repulsion (2000) pushes clusters apart
- Low center gravity (0.003) prevents clumping
- Long edge lengths (180-250px) give breathing room

**Don't reduce repulsion** or clusters will attract each other.

---

## Reference Implementation Notes

### TypeScript D3 Configuration

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

### Node Gradients (TypeScript)
```typescript
// Device (Blue)
grad.append("stop").attr("offset", "0%").attr("stop-color", "#60a5fa");
grad.append("stop").attr("offset", "100%").attr("stop-color", "#1d4ed8");

// Folder (Slate)
grad.append("stop").attr("offset", "0%").attr("stop-color", "#94a3b8");
grad.append("stop").attr("offset", "100%").attr("stop-color", "#334155");

// Group (Green)
grad.append("stop").attr("offset", "0%").attr("stop-color", "#34d399");
grad.append("stop").attr("offset", "100%").attr("stop-color", "#059669");

// Root (Purple)
grad.append("stop").attr("offset", "0%").attr("stop-color", "#c084fc");
grad.append("stop").attr("offset", "100%").attr("stop-color", "#7e22ce");
```

---

## Build & Run

```bash
dx build          # Build
dx serve          # Dev mode with hot reload
dx check          # Check without building
```

**NEVER use `cargo build`** - Dioxus projects must use `dx` commands.

---

## Testing Checklist

Before marking a feature complete:
- [ ] No infinite loops (monitor CPU, log file size)
- [ ] No console errors
- [ ] Works with 0 nodes (empty state)
- [ ] Works with 100+ nodes (performance)
- [ ] All interactions work (click, drag, lasso, etc.)
- [ ] State persists across restart (if applicable)
- [ ] No memory leaks (monitor over time)
- [ ] Viewport transform applied to all mouse coords
- [ ] Edges render behind nodes (z-index)
- [ ] Cluster backgrounds visible but faint

---

## Next Steps

### Immediate (P0)
1. Fix zoom functionality (wheel event API)
2. Complete edge creation (DB intent creation)
3. Directory expansion with filesystem scanning

### Short-term (P1)
4. Lasso multi-drag
5. Node visual polish (circles, gradients, sizes)
6. Node grouping

### Long-term (P2)
7. Performance optimization (1000+ nodes)
8. Edge status visualization
9. Smooth animations
