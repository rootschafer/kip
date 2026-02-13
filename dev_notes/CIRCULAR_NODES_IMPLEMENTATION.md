# Circular Directory Nodes Implementation Guide

## Status: PLANNED (NEXT FEATURE)

## Overview

**Goal**: Transform location nodes from rectangular "pills" to circles for directories/groups, while keeping files as pills. Implement two-level expansion (orbit view + enter view).

**Priority**: Second (after file picker, which is DONE)

**Complexity**: HIGH — involves:
- New node rendering logic (circle vs pill based on is_dir)
- State machine for expansion levels (collapsed → orbit → expanded)
- Layout math for arranging children in a ring
- SVG circle rendering + event handling
- Recursive expansion support

---

## Current State

From `src/ui/graph.rs`:
- **Nodes are rendered as rectangular "pills"** with label + handle
- **All nodes are the same shape** regardless of type
- **No expansion logic** — nodes are leaf-only
- All nodes laid out in a **single vertical column per container**

---

## What Must Change

### 1. Data Model Enhancement

**File: `src/ui/graph_types.rs`**

Extend `NodeView` structure to track:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct NodeView {
    pub id: RecordId,
    pub container_id: String,
    pub path: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub depth: usize,

    // NEW FIELDS:
    pub is_dir: bool,           // Directory = circle, File = pill
    pub is_expanded: bool,      // false = collapsed, true = expanded (inside view)
    pub is_orbit: bool,         // true = children fanned out around it (orbit view)
    pub child_count: usize,     // Number of direct children
}
```

**Key point**: `is_expanded`, `is_orbit` state MUST be stored in Dioxus signals in `MappingGraph`, NOT in the database (yet). We're not persisting expansion state per the current design.

### 2. Determine `is_dir` from Database

When loading locations in `load_nodes()`:

```rust
// Query includes path + a computed is_directory flag
// For local machine: check std::fs::metadata(path).is_dir()
// For remote machines: TODO (stored in DB or via SSH stat)
```

**Implementation approach**:
- In `load_nodes()`, after fetching location records, call an async helper `check_is_dir()` for each path
- For local machine: use `std::fs::metadata()` (blocking → `tokio::task::spawn_blocking`)
- For remote: stub it as `false` for now (no SSH yet)
- Cache results in memory during this refresh cycle

### 3. Expansion State Management

**File: `src/ui/graph.rs`**

Add to `MappingGraph` component:

```rust
// Map: node_id -> (is_expanded, is_orbit)
let mut expansion_state = use_signal(|| HashMap::<String, (bool, bool)>::new());
```

When a node is clicked:
1. Get current state: `expanded = expansion_state.with_untracked(|m| m.get(node_id))`
2. If `is_dir`:
   - Not expanded/orbit → switch to orbit view
   - In orbit view → switch to expanded view
   - In expanded view → collapse back to default
3. If file (pill): do nothing on click (or allow future context menu)

### 4. Circle Rendering (SVG)

**What changes in RSX**:

Instead of rendering all nodes as divs with class `"graph-node"`, branch based on `is_dir`:

**Files (pills)** — existing rectangular rendering:
```rsx
div {
    class: "graph-node",
    span { class: "node-label", "{label}" }
    div { class: "node-handle" }
}
```

**Directories (circles)** — new SVG circle rendering:
```rsx
svg { ... }  // Render circle via SVG, children in orbit/expanded views
```

### 5. Layout Math: Orbit View

When `is_orbit = true`, arrange **direct children in a ring around the parent circle**.

**Formula** (for N children):

```
parent_center = (cx, cy)
ring_radius = 80px (distance from parent center to child centers)
angle_per_child = 360° / N

for i in 0..N:
    angle = (360 / N) * i
    child_x = cx + ring_radius * cos(angle)
    child_y = cy + ring_radius * sin(angle)
    render_child_at(child_x, child_y)
```

**In Rust**:
```rust
fn compute_orbit_positions(parent_x: f64, parent_y: f64, children: &[NodeView])
    -> Vec<(usize, f64, f64)>
{
    const RING_RADIUS: f64 = 80.0;
    let n = children.len() as f64;
    let mut positions = Vec::new();

    for (i, child) in children.iter().enumerate() {
        let angle = (i as f64 / n) * 2.0 * std::f64::consts::PI;
        let x = parent_x + RING_RADIUS * angle.cos();
        let y = parent_y + RING_RADIUS * angle.sin();
        positions.push((i, x, y));
    }
    positions
}
```

Connect each child back to parent with a short edge (SVG line).

### 6. Layout Math: Expanded View

When `is_orbit = false` and `is_expanded = true`, the directory circle becomes a **container** and children stack inside it vertically (like how nodes currently stack in machine containers).

**Logic**:
- The circle grows into a rounded rectangle (or remains circular but with internal structure)
- Direct children are rendered inside at fixed positions (stacked vertically)
- The parent circle/box shows a label and `[collapse]` indicator
- A breadcrumb or "back" UI appears to collapse
- This is essentially a "zoom into" the directory

### 7. Recursion

Both children and parent can be directories. Clicking a child circle follows the same orbit → expand logic.

**State tracking**:
- Each node has independent `(is_expanded, is_orbit)` state
- A node can be expanded while its children are still in orbit
- This creates a tree-like navigation without global "zoom level" — each circle is independent

### 8. Click Handling

**File: `src/ui/graph.rs` — in node render section**

```rust
onmousedown: {
    let node_id = node_id_str.clone();
    move |e: MouseEvent| {
        if e.modifiers().shift() {
            // Shift+click: multi-select (existing logic)
        } else if node.is_dir {
            // Regular click on circle: toggle expansion
            e.stop_propagation();
            let mut state = expansion_state;
            let (expanded, orbit) = state.with_untracked(|m|
                m.get(&node_id).copied().unwrap_or((false, false))
            );

            // State machine:
            let new_state = if !expanded && !orbit {
                (false, true)   // collapsed → orbit
            } else if !expanded && orbit {
                (true, false)   // orbit → expanded
            } else {
                (false, false)  // expanded → collapsed
            };

            state.write().insert(node_id, new_state);
        } else {
            // Regular click on pill: start edge drag (existing)
            let coords = e.page_coordinates();
            *drag.write() = DragState::CreatingEdge { ... };
        }
    }
}
```

---

## Rendering Overview (Pseudo-code)

```javascript
for each container {
    render_container_header()

    // Load which nodes are direct children of this container (NOT nested under other nodes)
    let top_level_nodes = load_top_level_nodes(container_id)

    for each top_level_node {
        if node.is_dir {
            if node.is_orbit {
                render_circle_orbit(node, expansion_state)
            } else if node.is_expanded {
                render_circle_expanded(node, expansion_state)
            } else {
                render_circle_collapsed(node)
            }
        } else {
            render_pill_node(node)  // existing code
        }

        // Helper: recursively load + render children
        if node.is_expanded or node.is_orbit {
            let children = load_direct_children(node.id)
            render_children(children, node, expansion_state)  // RECURSIVE
        }
    }
}

fn render_circle_collapsed(node) {
    // Simple circle with label in center
    svg { class: "node-circle" ... }
}

fn render_circle_orbit(node, expansion_state) {
    // Parent circle + children arranged in ring
    svg { /* parent circle */ }
    children_positions = compute_orbit_positions(node.x, node.y, children)
    for (child, x, y) in children_positions {
        svg { /* child circle */ style: "... translate({x}px, {y}px)" }
        line { /* edge from child to parent */ }
    }
}

fn render_circle_expanded(node, expansion_state) {
    // Node becomes a container-like structure with children stacked inside
    // Top: circle shape with label + [collapse] button
    // Body: children stacked vertically
    div { class: "node-expanded-container" ... }
}
```

---

## CSS Changes Required

**File: `assets/main.css`**

### Circle Styling

```css
.node-circle {
    /* SVG circle for directories */
    fill: var(--glass-strong);
    stroke: var(--glass-border);
    stroke-width: 1;
    cursor: pointer;
    transition: fill 0.2s ease;
}
.node-circle:hover {
    fill: var(--glass-hover);
}
.node-circle.selected {
    fill: rgba(74, 158, 255, 0.15);
    stroke: var(--accent);
    stroke-width: 2;
}
.node-circle.has-children::after {
    content: "...";  /* or use CSS circles to show child count */
    font-size: 10px;
    font-weight: 600;
    color: var(--text-dim);
}

/* Orbit view: small children circles */
.node-circle-orbit {
    r: 24px;  /* smaller than parent */
}

/* Edge from child to parent in orbit view */
.orbit-edge {
    stroke: var(--glass-border);
    stroke-width: 1;
    opacity: 0.5;
    pointer-events: none;
}
```

### Expanded View

```css
.node-expanded-container {
    position: absolute;
    background: var(--glass-strong);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius);
    padding: 12px;
}
.node-expanded-header {
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
}
.node-expanded-body {
    display: flex;
    flex-direction: column;
    gap: 4px;
}
.node-expand-toggle {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--glass-hover);
    border: 1px solid var(--glass-border);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 10px;
    transition: all 0.15s ease;
}
.node-expand-toggle:hover {
    background: var(--accent);
    border-color: var(--accent);
}
```

---

## Database Queries (No Schema Changes)

**is_dir detection** (in `load_nodes()` after fetching locations):

```rust
async fn enrich_locations_with_is_dir(
    locations: Vec<LocationRow>,
    containers: &[ContainerView],
) -> Vec<(LocationRow, bool)> {
    let mut results = Vec::new();
    for loc in locations {
        let is_dir = if let Some(container) = containers.iter().find(|c| c.id == loc.machine || c.id == loc.drive) {
            // Local machine
            if container.kind == "local" {
                tokio::task::spawn_blocking({
                    let path = loc.path.clone();
                    move || std::path::Path::new(&path).is_dir()
                })
                .await
                .unwrap_or(false)
            } else {
                false  // Remote: stub for now
            }
        } else {
            false
        };
        results.push((loc, is_dir));
    }
    results
}
```

---

## Gotchas & Warnings

1. **Nested expansion state is HARD**: If parent is expanded and child is also expanded, how do they nest visually? Current design: expanded children stack inside expanded parent. Need layout math to prevent overlap.

2. **Refresh cycle**: When `refresh_tick` changes (user adds location, edge created, etc.), graph reloads from DB. **Expansion state MUST survive this** because it's in a Dioxus signal, not the DB. But positions might change if new nodes are added, so expanded children might reflow.

3. **SVG coordinate space**: SVG children must be positioned in the same coordinate system as HTML nodes. Use `translate()` transforms or absolute positioning calcs.

4. **Performance**: If a directory has 100 children and user expands it, rendering 100+ nodes + layout calcs could lag. Defer: virtualization is a future optimization.

5. **Breadcrumbs**: When expanded, show a breadcrumb "Container / Directory / File" path at the top of the expanded view, or a simple "back" button.

6. **Re-rendering efficiency**: Each node click triggers `expansion_state.write()`, which rerenders `MappingGraph`. If there are 20 nodes, all rerender even though only one changed. Use `with_untracked()` and `write()` carefully to minimize full-tree rerenders.

---

## Implementation Order

1. **Data model** (`graph_types.rs`): Add `is_dir`, `is_expanded`, `is_orbit`, `child_count` to `NodeView`

2. **Expansion state signal** (`graph.rs`): Add `HashMap<String, (bool, bool)>` signal for tracking expansion per node

3. **Load is_dir** (`graph.rs`): Modify `load_nodes()` to detect directories for local machine paths

4. **Click handling** (`graph.rs`): Add click handler logic to toggle expansion state

5. **Orbit layout** (`graph_types.rs`): New function `compute_orbit_positions()`

6. **Circle rendering** (`graph.rs` RSX): Render circles for directories, pills for files, orbit/expanded layouts

7. **CSS** (`main.css`): Add styles for `.node-circle`, `.node-expanded-container`, etc.

8. **Test**: Run `dx serve`, click directories, verify orbit/expand views work

---

## Future Enhancements (NOT for this pass)

- [ ] Persistent expansion state (save to DB)
- [ ] Animation when transitioning orbit → expand
- [ ] Child count badge on circle
- [ ] Drag-to-expand: drag a child out of parent circle
- [ ] Double-click to expand (vs. click-once-for-orbit, click-twice-for-expand)
- [ ] Right-click context menu: "Expand all children", "Collapse all"
- [ ] Breadcrumb/back navigation for deep hierarchies
- [ ] Force-directed layout instead of columnar for large graphs with many levels

---

## References

- `KIP_DESIGN_7_MAPPING_GRAPH.md` — Section "Node Shapes: Circles vs. Pills" and "Directory/Group Circle Interaction"
- `src/ui/graph.rs` — Current node rendering logic (lines 342-420)
- `src/ui/graph_types.rs` — `NodeView`, layout helpers, bezier path functions
- `assets/main.css` — Existing glassmorphic node styling (.graph-node, .graph-node.selected, etc.)
