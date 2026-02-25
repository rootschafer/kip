# Next Agent Handoff

**Date:** February 22, 2026  
**Priority:** HIGH — Interaction Model Implementation

---

## What Changed Today

### Documentation Updates

**New Documents:**
- `INTERACTION_MODEL.md` — Complete specification for click/drag/double-click behavior and context menus
- Updated `COMPREHENSIVE_DEVELOPMENT_PLAN.md` — Accurate current state, expanded long-term vision
- Updated `START_HERE.md` — Entry point for new developers
- Updated `IMPLEMENTATION_SUMMARY.md` — What's implemented vs. not

**Architecture Docs:**
- `new_arch/*` — Complete API layer and CLI documentation

### Code Changes

**Fixed:**
- SurrealDB type coercion (RecordId → String in IntentRow)
- Simulation restart logic (no longer restarts on every refresh tick)
- Cluster background circles removed
- Debug CSS and test nodes removed

**Not Fixed (Design Complete):**
- Click/drag conflict — See `INTERACTION_MODEL.md`
- Context menus — See `INTERACTION_MODEL.md`
- Keyboard shortcuts — See `INTERACTION_MODEL.md`

---

## Your Task: Implement New Interaction Model

### Overview

The current interaction model has a fundamental conflict:
- **Single click** both selects AND starts drag
- This makes precise selection difficult
- No context menus for node operations

The new model (designed, not implemented):
1. **Single click** → Select only
2. **Click + drag** → Move node(s)
3. **Double click** → Open context menu

### Files to Modify

| File | Changes Needed |
|------|----------------|
| `src/ui/graph.rs` | Mouse event handlers, context menu rendering |
| `src/ui/graph_nodes.rs` | Node click handling, selection highlighting |
| `src/ui/graph_store.rs` | Selection state, context menu state |
| `assets/main.css` | Context menu styling, selection highlighting |

### Implementation Steps

#### 1. Fix Click Behavior (`graph.rs`)

**Current:**
```rust
onmousedown: move |e: MouseEvent| {
    // Starts drag immediately
    graph.with_mut(|g| {
        g.drag_state = DragState::Dragging { ... };
    });
}
```

**New:**
```rust
onmousedown: move |e: MouseEvent| {
    // Just select the node
    graph.with_mut(|g| {
        g.select_node(&node_id);
    });
}

onmouseup: move |e: MouseEvent| {
    // Check if it was a click (no movement)
    if distance_moved < THRESHOLD {
        // Single click - just selection (already done)
    }
}

// Track movement during drag
onmousemove: move |e: MouseEvent| {
    if distance_moved > THRESHOLD {
        // Now start drag
        graph.with_mut(|g| {
            g.drag_state = DragState::Dragging { ... };
        });
    }
}
```

#### 2. Add Double-Click Handler

```rust
ondblclick: move |e: MouseEvent| {
    e.stop_propagation();
    graph.with_mut(|g| {
        g.show_context_menu(&node_id, e.client_coordinates());
    });
}
```

#### 3. Implement Context Menu

**State:**
```rust
pub enum ContextMenuState {
    Hidden,
    Visible {
        node_id: String,
        x: f64,
        y: f64,
        node_type: NodeType,
    },
}
```

**Rendering:**
```rust
if let Some(menu) = graph().context_menu {
    rsx! {
        div {
            class: "context-menu",
            style: "left: {menu.x}px; top: {menu.y}px;",
            // Render menu items based on node_type
        }
    }
}
```

**Menu Items:**
```rust
const MACHINE_MENU_ITEMS: &[&str] = &[
    "Into (ENTER)",
    "Expand (SPACE)",
    "─",
    "Tag",
    "Move (M)",
    "Copy",
    "Copy Path",
    "Open in Finder",
    "─",
    "Delete",
];
```

#### 4. Add Keyboard Shortcuts

```rust
// In graph.rs, add keyboard event handler
onkeydown: move |e: KeyboardEvent| {
    match e.key().as_str() {
        "Enter" => {
            // Primary action (Into/Open)
            if let Some(selected) = graph().selected.first() {
                graph.with_mut(|g| g.activate_node(selected));
            }
        }
        " " => {
            // Expand (orbit view)
            if let Some(selected) = graph().selected.first() {
                graph.with_mut(|g| g.expand_node(selected));
            }
        }
        "Delete" => {
            // Delete selected nodes
            graph.with_mut(|g| g.delete_selected());
        }
        "Escape" => {
            // Deselect all, close menus
            graph.with_mut(|g| g.deselect_all());
        }
        _ => {}
    }
}
```

### Reference Documents

- `INTERACTION_MODEL.md` — Complete interaction specification
- `KIP_DESIGN_7_MAPPING_GRAPH.md` — Graph UI architecture (still relevant)
- `COMPREHENSIVE_DEVELOPMENT_PLAN.md` — Phase 1 priorities

---

## Testing

After implementation:

1. **Single click** should only select (no drag start)
2. **Click + drag** should move node
3. **Double click** should open context menu
4. **Keyboard shortcuts** should work (ENTER, SPACE, DELETE, ESC)
5. **Context menu** should have correct items for each node type

---

## Other Pending Tasks

### After Interaction Model (Phase 2)

1. **Orbit view** — Children fan out around parent circle
2. **Enter view** — Navigate into directory context
3. **Node grouping** — Collapse multiple nodes
4. **Layout persistence** — Save/restore positions

### Long-term (Phase 3+)

1. **Bidirectional sync**
2. **Scheduled intents**
3. **SSH/SFTP support**
4. **Web frontend**

See `COMPREHENSIVE_DEVELOPMENT_PLAN.md` for full roadmap.

---

## Questions?

- **Interaction design:** Read `INTERACTION_MODEL.md`
- **Current state:** Read `IMPLEMENTATION_SUMMARY.md`
- **Architecture:** Read `new_arch/README.md`
- **Bugs:** Check `CRITICAL_ISSUES.md`

---

## Document History

| Date | Change |
|------|--------|
| 2026-02-17 | Initial handoff |
| 2026-02-22 | Major revision: new interaction model, accurate state |

