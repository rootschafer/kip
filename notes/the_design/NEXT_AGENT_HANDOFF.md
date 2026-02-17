# AI Handoff Prompt: Kip Force-Directed Graph Development

## Your Task

Continue development of the **force-directed graph UI** for the Kip file synchronization app. The core physics system is working - you need to complete interaction features and polish.

---

## 🚨 CRITICAL: Read These First

**DO NOT start coding until you have:**

1. ✅ Read `notes/the_design/START_HERE.md` - Project overview and critical pitfalls
2. ✅ Read `notes/the_design/CRITICAL_ISSUES.md` - Known bugs and infinite loop fixes  
3. ✅ Read `notes/the_design/IMPLEMENTATION_SUMMARY.md` - Current implementation state
4. ✅ Studied `external/nexus-node-sync/components/GraphCanvas.tsx` - D3 force-directed reference
5. ✅ Studied `external/nexus-node-sync/App.tsx` - Interaction logic reference
6. ✅ Understood the infinite loop patterns that broke the app before

**IF YOU SKIP THESE STEPS, YOU WILL BREAK THE APP.**

---

## Project Context

**What is Kip?**
Kip is a file synchronization orchestrator. Users connect devices (local machines, remote servers via SSH, mounted drives) and create sync relationships between folders. The primary UI is a force-directed graph where:
- **Nodes** = devices, folders, files
- **Edges** = sync relationships
- **Interactions** = drag to move, click to expand, drag-between-nodes to create sync

**Tech Stack:**
- **Frontend:** Dioxus 0.7.3 (Rust React-like framework)
- **Backend:** SurrealDB (embedded database)
- **Graphics:** SVG overlay for edges, HTML divs for nodes
- **State:** Dioxus signals and stores

**Current State:**
- ✅ Force-directed layout WORKING (nodes spread, clusters separate)
- ✅ Infinite canvas with Alt+drag pan
- ✅ Filesystem scanning on machine/drive expansion
- ✅ All nodes connected with hierarchy edges
- ✅ Edge preview line aligns with cursor
- ✅ Cluster backgrounds (faint colored circles)
- ❌ Zoom NOT working (Dioxus wheel API issue)
- ❌ Directory expansion incomplete (only machines/drives scan)
- ❌ Edge creation incomplete (UI exists, no DB creation)
- ❌ Lasso multi-drag not implemented
- ❌ Node visuals basic (no circles/gradients/status)

---

## Reference Implementation

**Location:** `external/nexus-node-sync/`

This is a complete TypeScript/React implementation using D3.js. Study these files:

### 1. `types.ts` - Data Structures
```typescript
interface FileNode {
  id: string;
  name: string;
  type: 'root' | 'device' | 'folder' | 'file' | 'group';
  x?: number;
  y?: number;
  fx?: number | null;  // Fixed position (during drag)
  fy?: number | null;
}
```

**Your equivalent:** `src/ui/graph_types.rs` - `GraphNode` struct (has fx, fy, vx, vy)

### 2. `GraphCanvas.tsx` - Force-Directed Graph
Key D3 configuration:
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

**Your equivalent:** `src/ui/graph_store.rs` - `apply_forces()` function

**Your current constants (tuned for cluster separation):**
```rust
REPULSION: 2000.0      // Stronger than D3 (-300)
SPRING_K: 0.03         // Weaker than D3
CENTER_GRAVITY: 0.003  // Much weaker than D3 (0.04)
```

### 3. `App.tsx` - Interaction Logic
Key patterns:
- Click node → expand if directory
- Shift+click → multi-select
- Link mode → create sync edge
- Drag node → move with physics
- Shift+drag background → lasso select

**Your equivalent:** `src/ui/graph.rs` - mouse event handlers

---

## Implementation Priorities

### P0: Complete Core Interactions (4-6 hours)

#### 1. Fix Zoom Functionality
**Status:** BLOCKED - Dioxus API incompatibility

**Problem:**
```rust
// All these fail in Dioxus 0.7.3:
e.delta_y()           // Method doesn't exist
e.data().delta_y()    // Method doesn't exist  
e.data().y            // Field doesn't exist
```

**Approaches:**
- **Option A:** Find correct Dioxus wheel API (check docs/examples)
- **Option B:** Add zoom buttons (+/-) in toolbar
- **Option C:** Keyboard zoom (Ctrl+scroll or +/- keys)

**Reference:** `GraphCanvas.tsx` uses `d3.zoom()` with wheel filter

#### 2. Complete Edge Creation
**Status:** PARTIAL - UI exists, DB creation missing

**What Works:**
- Ctrl/Alt+click starts edge creation
- Rubber band line follows cursor (viewport-transformed)
- `DragState::CreatingEdge` tracks state

**What's Missing:**
- Drop on target node doesn't complete edge
- No intent created in database
- No visual feedback on hover

**Implementation:**
```rust
// In node mousedown handler, check for CreatingEdge state:
if let DragState::CreatingEdge { source_id } = &graph().drag_state {
    // Complete the edge
    let new_edge = GraphEdge {
        id: format!("edge_{}_{}", source_id, node_id),
        source_id: source_id.clone(),
        dest_id: node_id.clone(),
        status: "idle".to_string(),
        // ... other fields
    };
    
    // Create in database
    spawn(async move {
        let _ = create_edge_in_db(&db, source_id, &node_id).await;
    });
    
    // Add to graph
    graph.with_mut(|g| {
        g.edges.push(new_edge);
        g.drag_state = DragState::None;
        g.start_simulation();
    });
}
```

**Test:**
- Ctrl+click node A
- Drag to node B
- Release on B
- Edge should appear and persist

#### 3. Directory Expansion with Filesystem Scan
**Status:** PARTIAL - Works for machines/drives, not directories

**Current Flow:**
```
Click machine/drive → scan_directory() → nodes appear in orbit
```

**Missing:**
```
Click directory → ??? → children should appear
```

**Implementation:**
Extend the expansion handler in `graph.rs`:
```rust
if kind.is_expandable() {
    let node_info = graph().find_node(&node_id)
        .map(|n| (n.kind.clone(), n.path.clone(), n.parent_id.clone()));
    
    match node_info {
        Some((NodeKind::Directory { .. }, path, parent_id)) => {
            // Check if children exist in DB
            let has_children = graph().nodes.iter()
                .any(|n| n.parent_id.as_ref() == Some(&node_id));
            
            if !has_children && !path.is_empty() {
                // Scan filesystem for this directory
                spawn(async move {
                    let nodes = scan_directory(&db, &node_id, &path, x, y).await?;
                    graph_signal.with_mut(|g| {
                        g.complete_filesystem_scan(&node_id, nodes);
                    });
                });
            }
        }
        _ => {}
    }
    
    graph.with_mut(|g| g.toggle_expand(&node_id));
}
```

**Test:**
- Expand machine (works - scans filesystem)
- Click directory child (should scan that directory's contents)
- Children should appear in orbit around directory

---

### P1: Polish & UX (3-4 hours)

#### 4. Lasso Multi-Drag
**Status:** PARTIAL - Selection works, multi-drag missing

**What Works:**
- Shift+drag creates selection rectangle
- `select_in_rect()` selects nodes

**What's Missing:**
- Selected nodes don't move together
- Only single-node drag works

**Implementation:**
In `DragState::Dragging` handler:
```rust
let selected: Vec<String> = graph().selected.iter().cloned().collect();
for id in selected {
    if let Some(node) = graph.find_node_mut(&id) {
        node.fx = Some(new_x + offset_x);
        node.fy = Some(new_y + offset_y);
        node.position = Vec2::new(new_x + offset_x, new_y + offset_y);
    }
}
```

#### 5. Node Visual Design
**Status:** BASIC - All nodes use same rectangular styling

**Missing:**
- Circle nodes for directories/groups
- Size based on descendant count
- Status indicators (synced, syncing, error)
- Selection glow
- Gradient fills

**Reference from TypeScript:**
```typescript
// Node sizes
device: 24px radius
folder/group: 18px radius
file: 8px radius

// Gradients
device: #60a5fa → #1d4ed8 (blue)
folder: #94a3b8 → #334155 (slate)
group: #34d399 → #059669 (green)
root: #c084fc → #7e22ce (purple)
```

**Implementation:**
Update CSS in `src/ui/graph_nodes.rs`:
```rust
// Circle for directories
if matches!(node.kind, NodeKind::Directory { .. } | NodeKind::Group { .. }) {
    rsx! {
        div {
            class: "graph-node circle",
            style: "
                width: {size}px;
                height: {size}px;
                border-radius: 50%;
                background: radial-gradient(circle, {color_start}, {color_end});
            ",
        }
    }
}
```

#### 6. Status Indicators
Add visual feedback for node states:
- **Syncing:** Pulsing border animation
- **Error:** Red X overlay
- **Offline:** Grayed out, dashed border
- **Selected:** Blue glow with dashed border

---

### P2: Performance & Edge Cases (2-3 hours)

#### 7. Performance with 100+ Nodes
**Current:** Should work, untested at scale

**Optimization if needed:**
- Barnes-Hut approximation for repulsion (O(n log n) vs O(n²))
- Spatial hashing for collision detection
- Limit visible nodes (virtual scrolling)

#### 8. Edge Cases
- [ ] Empty state (no machines)
- [ ] Single node
- [ ] 1000+ nodes
- [ ] Disconnected machine (show offline state)
- [ ] Very long paths (truncate labels)

---

## File Locations

### Core Files to Modify
- `src/ui/graph_store.rs` - Graph struct, physics, scanning
- `src/ui/graph.rs` - Component, mouse handlers, simulation loop
- `src/ui/graph_types.rs` - Type definitions
- `src/ui/graph_nodes.rs` - Node rendering
- `src/ui/graph_edges.rs` - SVG edge overlay

### Reference Files
- `external/nexus-node-sync/components/GraphCanvas.tsx` - D3 implementation
- `external/nexus-node-sync/App.tsx` - Interaction logic
- `external/nexus-node-sync/types.ts` - Type definitions

### Documentation
- `notes/the_design/START_HERE.md` - Project overview
- `notes/the_design/CRITICAL_ISSUES.md` - Known bugs
- `notes/the_design/IMPLEMENTATION_SUMMARY.md` - Current state

---

## ⚠️ CRITICAL: Infinite Loop Prevention

**YOU MUST FOLLOW THESE RULES OR THE APP WILL FREEZE:**

### Rule 1: Always Wrap Spawns in use_effect

```rust
// ❌ WRONG - Creates new spawn on EVERY render
#[component]
fn MyComponent() -> Element {
    spawn(async move {
        loop {
            do_something().await;
        }
    });
    rsx! { /* ... */ }
}

// ✅ CORRECT - Spawns only once
#[component]
fn MyComponent() -> Element {
    use_effect(move || {
        spawn(async move {
            loop {
                do_something().await;
            }
        });
    });
    rsx! { /* ... */ }
}
```

### Rule 2: Don't Update Signals from Inside use_resource

```rust
// ❌ WRONG - Resource updates signal, triggers re-render, recreates resource
use_resource(move || {
    let graph_val = graph.clone();
    async move {
        let data = load().await;
        graph_val.with_mut(|g| g.load(data)); // TRIGGERS INFINITE LOOP
    }
});

// ✅ CORRECT - Separate resource and effect
let loaded_data = use_resource(move || {
    async move { load().await.ok() }
});

use_effect(move || {
    if let Some(Some(data)) = loaded_data.read().as_ref() {
        graph.with_mut(|g| g.load(data));
    }
});
```

### Rule 3: Capture Values, Not Signals

```rust
// ❌ WRONG - Captures Signal, closure changes every render
use_resource(move || {
    let tick = refresh_tick; // tick is Signal<u32>
    async move { /* ... */ }
});

// ✅ CORRECT - Capture the VALUE
use_resource(move || {
    let tick = refresh_tick; // tick is u32 (the VALUE)
    async move {
        let _ = tick; // Use the value
        /* ... */
    }
});
```

### Rule 4: No File Logging

```rust
// ❌ WRONG - Created 209GB log file
tracing_subscriber::registry()
    .with(tracing_subscriber::fmt().layer().with_writer(file_appender))
    .init();

// ✅ CORRECT - Console only, WARN level
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::WARN)
    .init();
```

**Already fixed in:** `src/main.rs`

---

## Testing Checklist

After each feature:

- [ ] No infinite loops (run for 2+ minutes)
- [ ] CPU usage reasonable (< 20% when idle)
- [ ] No console errors
- [ ] Log file not growing (`ls -lh ~/Library/Application\ Support/Kip/kip.log`)
- [ ] Interactions work as expected
- [ ] Visual looks correct
- [ ] Viewport transform applied to all mouse coords
- [ ] Edges render behind nodes (z-index: 1)

**If CPU spikes or log file grows:**
1. Kill app immediately
2. Check what you changed
3. Review infinite loop patterns in `CRITICAL_ISSUES.md`
4. Fix before continuing

---

## Build Commands

```bash
# Build the app
dx build

# Run in development mode
dx serve --platform desktop

# Check without building
dx check
```

**DO NOT use `cargo build` or `cargo check`** - Dioxus projects must use `dx` commands.

---

## Debugging Tips

### Detecting Infinite Loops

1. **Watch CPU usage** - Spikes to 100% immediately
2. **Watch log file size** - Grows rapidly (GBs per minute)
3. **Add counter logging:**
   ```rust
   static COUNTER: AtomicUsize = AtomicUsize::new(0);
   let count = COUNTER.fetch_add(1, Ordering::Relaxed);
   if count % 100 == 0 { tracing::info!("Render {}", count); }
   ```

### Viewport Issues

If nodes don't align with mouse:
```rust
// Always transform screen → graph space:
let graph_x = (mouse_x - viewport_x) / viewport_scale;
let graph_y = (mouse_y - viewport_y) / viewport_scale;
```

### Force Tuning

If clusters clump together:
- Increase `REPULSION` (currently 2000)
- Decrease `CENTER_GRAVITY` (currently 0.003)
- Increase edge lengths (currently 180-250px)

---

## When You Get Stuck

1. **Check documentation first:**
   - `START_HERE.md` for general patterns
   - `CRITICAL_ISSUES.md` for bugs
   - TypeScript reference for expected behavior

2. **Compare with TypeScript reference:**
   - How does D3 handle this?
   - Can you port the logic directly?

3. **Document new issues:**
   - Add to `CRITICAL_ISSUES.md`
   - Include reproduction steps
   - Include fix if found

4. **Update documentation:**
   - When you fix something, update the docs
   - When you add a feature, update `IMPLEMENTATION_SUMMARY.md`
   - Help the next person (or future you)

---

## Success Criteria

Your implementation is complete when:

1. **Zoom Works:**
   - Scroll wheel or buttons zoom toward cursor
   - Range: 0.1x to 5.0x
   - Smooth, no jank

2. **Edge Creation Works:**
   - Ctrl+click node A, drag to node B, release
   - Edge appears and persists in database
   - Visual feedback during drag

3. **Directory Expansion Works:**
   - Click directory → filesystem scans
   - Children appear in orbit
   - Works recursively (expand children too)

4. **Lasso Multi-Drag Works:**
   - Shift+drag selects multiple
   - Drag moves all selected together
   - Release settles with physics

5. **Visual Polish:**
   - Circle nodes for directories/groups
   - Pill nodes for files
   - Gradients match reference
   - Status indicators visible

6. **No Infinite Loops:**
   - App runs for 5+ minutes without freezing
   - CPU stays reasonable
   - Log file stays small (< 10MB)

7. **Performance:**
   - 60fps with 50 nodes
   - 30+ fps with 200 nodes
   - No memory leaks

---

## Good Luck!

You have everything you need:
- ✅ Working force-directed physics
- ✅ Infinite canvas with pan
- ✅ Filesystem scanning infrastructure
- ✅ Reference implementation
- ✅ Detailed documentation
- ✅ Known pitfalls documented

**Just follow the rules, test frequently, and document as you go.**

If you hit an issue, it's probably documented in `CRITICAL_ISSUES.md`. If it's a NEW issue, add it there so the next person doesn't hit the same problem.

**Happy coding! 🚀**
