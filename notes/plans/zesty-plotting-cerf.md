# Force-Directed Graph Overhaul

## Context

Kip's graph UI currently uses a grid-based layout with an orbit/enter expansion model that is overly complex and fundamentally wrong. The right approach is a force-directed graph where **all layout is handled by physics simulation**. Every interaction (expand directory, add node, group nodes) reduces to "change node properties, let the algorithm place everything." This overhaul replaces the grid layout, orbit/enter system, and container-based rendering with a single `Store<Graph>` and a from-scratch force simulation.

Additionally: a `NotificationService` store for toast messages, and a `safe_mode` cargo feature to prevent accidental file operations during development.

## Files to Create

| File | Purpose |
|------|---------|
| `src/ui/graph_store.rs` | `Store<Graph>` — all graph state, force simulation, graph API |
| `src/ui/graph_nodes.rs` | Node rendering components: `FileNode`, `DirNode`, `MachineNode`, `DriveNode` |
| `src/ui/graph_edges.rs` | `GraphSvgOverlay` — edges, rubber band, lasso |
| `src/ui/notification.rs` | `Store<NotificationService>` + `NotificationLayer` component |

## Files to Modify

| File | Changes |
|------|---------|
| `src/ui/graph_types.rs` | Replace `NodeView`/`EdgeView` with `Vec2`, `NodeKind`, `GraphNode`, `GraphEdge`. Keep `ContainerView`, `bezier_path`, `edge_color`, `palette_color`, `short_path`, `path_contains`. |
| `src/ui/graph.rs` | Gut and rewrite — thin component wiring `Store<Graph>` to simulation loop + rendering. ~200 lines instead of ~1100. |
| `src/ui/container_components.rs` | Remove `WorkspaceNode`, `NodeHandle`. Keep `MachineChip`. |
| `src/ui/mod.rs` | Add `graph_store`, `graph_nodes`, `graph_edges`, `notification`. Remove `picker_store`. |
| `src/app.rs` | Create `Store<Graph>` and `Store<NotificationService>` via `use_store`. Render `NotificationLayer`. |
| `src/engine/copier.rs` | Wrap file I/O in `#[cfg(not(feature = "safe_mode"))]` |
| `src/engine/scanner.rs` | Wrap filesystem walking in `#[cfg(not(feature = "safe_mode"))]` |
| `Cargo.toml` | Add `safe_mode` feature (default) |
| `assets/main.css` | Add notification toast styles. Remove dead container CSS. Remove/shorten `.ws-node` position transition (simulation updates 60fps, CSS transition lags). |

## Files to Delete

| File | Reason |
|------|--------|
| `src/ui/picker_store.rs` | Dead duplicate of `PickerManager` from `file_picker.rs` |

## Implementation Phases

### Phase 0: Groundwork (non-breaking)
1. Add `safe_mode` feature to `Cargo.toml`: `default = ["desktop", "safe_mode"]`, `safe_mode = []`
2. Add `#[cfg(not(feature = "safe_mode"))]` guards to file-modifying functions in `copier.rs` and `scanner.rs`
3. Delete `src/ui/picker_store.rs`
4. Fix deprecation warnings (`ReadOnlySignal` -> `ReadSignal` in `graph.rs:70`)
5. Prefix unused variables with `_`

### Phase 1: NotificationService
6. Create `src/ui/notification.rs`:
   - `NotificationService` store with `notifications: Vec<Notification>`, `next_id: u32`
   - `Notification` struct: `id`, `message`, `created_at` (use `std::time::Instant`), `state` (Active/Dismissed/TimedOut)
   - `#[store(pub)] impl Store<NotificationService>` with `add_notification(message: String)`, `dismiss(id: u32)`
   - `NotificationLayer` component: fixed bottom-right, renders active notifications stacked upward, auto-timeout ~5s via `use_future`
7. Add to `app.rs` and `mod.rs`

### Phase 2: Data Structures (`graph_types.rs` rewrite)
8. Add `Vec2` struct with `x: f64, y: f64`, arithmetic ops (`Add`, `Sub`, `Mul<f64>`, `AddAssign`, `SubAssign`), `length()`, `normalized()`
9. Add `NodeKind` enum: `File`, `Directory { expanded: bool }`, `Group { expanded: bool }`, `Machine`, `Drive { connected: bool }`
10. Add `GraphNode` struct:
    - `id: String`, `label: String`, `path: String`, `kind: NodeKind`
    - `parent_id: Option<String>` (machine/drive that owns this location)
    - `color: String` (palette color from parent)
    - `position: Vec2`, `velocity: Vec2`
    - `pinned: bool`, `visible: bool`
    - `width: f64`, `height: f64`
11. Add `GraphEdge` struct: `id: String`, `source_id: String`, `dest_id: String`, `status: String`, `total_files: i64`, `completed_files: i64`
12. Remove old `NodeView`, `EdgeView`, `compute_depth`, `compute_orbit_positions`

### Phase 3: Graph Store (`graph_store.rs`)
13. `Graph` struct with `#[derive(Store, Clone, PartialEq)]`:
    - `nodes: Vec<GraphNode>`, `edges: Vec<GraphEdge>`
    - `alpha: f64` (simulation energy), `sim_running: bool`
    - `viewport_offset: Vec2`, `zoom: f64`
    - `selected: HashSet<String>`, `drag_state: DragState`
    - `containers: Vec<ContainerView>`, `review_count: i64`
14. `#[store(pub)] impl Store<Graph>` methods:
    - `add_node`, `remove_node`, `set_visible`, `toggle_expand`
    - `set_position` (for drag, sets `pinned: true`)
    - `add_edge`, `remove_edge`
    - `tick` (one simulation step)
    - `load_from_db` (bulk load, sets alpha=1.0)
    - `start_sim` (sets alpha, sim_running=true)
15. `apply_forces(nodes, edges, alpha)` standalone function:
    - **Repulsion**: Coulomb's law between all visible node pairs, `F = 500 / d^2`
    - **Edge spring**: Hooke's law, `F = 0.05 * (d - 120)`, connected nodes attract
    - **Parent spring**: Stronger pull (`0.08`) between location nodes and their machine/drive parent, rest length 80px — creates natural clustering
    - **Center gravity**: `0.01 * (center - position)` gentle centering
    - **Damping**: `velocity *= 0.9`
    - **Alpha decay**: `alpha *= 0.995` per tick
    - **Pinned nodes**: exert forces on others but don't move themselves
16. `load_graph_data(db)` async function — moved from `graph.rs`, returns `(Vec<ContainerView>, Vec<GraphNode>, Vec<GraphEdge>, i64)`
    - Machines become `NodeKind::Machine` nodes
    - Drives become `NodeKind::Drive` nodes
    - Locations become `File` or `Directory` nodes with `parent_id` set
    - Intents become edges
    - Locations with saved `graph_x`/`graph_y` get those positions + `pinned: true`
    - Top-level locations (added by user) start `visible: true`; their children start `visible: false`

### Phase 4: Node Components (`graph_nodes.rs`)
17. `GraphNodeComponent` — dispatches to the right renderer based on `NodeKind`
18. `FileNode` — pill shape, shows label, monospace font
19. `DirNode` — circle, shows label + child count, click calls `graph.toggle_expand(id)`
20. `MachineNode` — larger circle with machine name, colored border
21. `DriveNode` — circle with drive name, dimmed if disconnected
22. All nodes handle:
    - **Left-click (no mod)**: `LeftClickPending` -> if <5px movement = click action (expand for dirs, notification for files), if >5px = drag (updates position via `graph.set_position`)
    - **Shift-click**: toggle selection
    - **Ctrl/Alt-click**: start edge creation
    - On mouseup during edge creation on another node: `graph.add_edge(...)` + DB create

### Phase 5: Edge Rendering (`graph_edges.rs`)
23. `GraphSvgOverlay` component — SVG positioned over workspace
24. Renders bezier paths for all edges where both source and dest are visible
25. Renders rubber-band line during `DragState::CreatingEdge`
26. Renders lasso rectangle during `DragState::Lasso`
27. Edge colors from `edge_color()` based on status

### Phase 6: Main Component Rewrite (`graph.rs`)
28. New `MappingGraph` component:
    - Takes `Store<Graph>` as prop
    - `use_resource` to load data from DB on mount + refresh_tick, calls `graph.load_from_db(...)`
    - Simulation loop via `use_future`: checks `graph.sim_running()`, calls `graph.tick()` every 16ms, sleeps when not running
    - Renders: `GraphToolbar` (from graph.containers/review_count), workspace div with `GraphSvgOverlay` + node components for each visible node
    - Workspace mousedown/mousemove/mouseup for pan, lasso, deselect
29. Remove all old layout constants, `expansion_state` HashMap, `calculate_orbit_positions`, `calculate_node_size`, `get_visible_nodes`, old `load_nodes`/`load_edges`/`load_containers`

### Phase 7: Integration & Cleanup
30. Update `container_components.rs` — remove `WorkspaceNode`, `NodeHandle`, keep `MachineChip`
31. Update `app.rs` — create stores, pass to components, render `NotificationLayer`
32. Update `mod.rs` — new modules
33. Update `main.css` — notification styles, remove dead CSS, set `.ws-node { transition: none; }` for position (simulation handles animation)
34. Clean unused imports, dead code across all modified files

### Phase 8: Polish
35. Persist positions: on node drag end, save `graph_x`/`graph_y` to location record in SurrealDB
36. Tune force constants through testing
37. Add pan (middle-click drag or scroll) and zoom (scroll wheel) to workspace

## Force-Directed Algorithm

```
Constants:
  REPULSION = 500.0       # Coulomb repulsion strength
  SPRING_K = 0.05         # Edge spring constant
  SPRING_REST = 120.0     # Edge rest length (px)
  PARENT_K = 0.08         # Parent-child spring (stronger)
  PARENT_REST = 80.0      # Parent-child rest length
  CENTER_G = 0.01         # Center gravity
  DAMPING = 0.9           # Velocity damping per tick
  ALPHA_DECAY = 0.995     # Energy decay per tick
  ALPHA_MIN = 0.001       # Stop threshold
  WARM_RESTART = 0.3      # Alpha on graph mutation

Per tick:
  1. Repulsion: every visible pair pushes apart (F = REPULSION / d^2)
  2. Edge springs: connected visible nodes attract (F = SPRING_K * (d - SPRING_REST))
  3. Parent springs: location<->machine/drive attract (F = PARENT_K * (d - PARENT_REST))
  4. Center gravity: all nodes pulled gently to center
  5. Apply: velocity += force * alpha; velocity *= DAMPING; position += velocity
  6. Decay: alpha *= ALPHA_DECAY; if alpha < ALPHA_MIN, stop simulation

Pinned nodes: receive no forces, but exert forces on others
```

## Key Architecture Decisions

- **No containers**: Machines and drives are regular nodes in the force graph. Parent-child springs create natural visual clustering without explicit container rendering.
- **Visibility = expansion**: Clicking a directory just toggles `visible` on its direct children. The force simulation handles placement.
- **Single Store**: All graph state in one `Store<Graph>`. Methods like `toggle_expand(id)` encapsulate multi-step state changes.
- **Simulation loop**: `use_future` with `tokio::time::sleep(16ms)`. Triggered by setting `alpha > 0` and `sim_running = true`. Stops when alpha decays below threshold.
- **Position persistence**: `graph_x`/`graph_y` already exist on `location` table in SurrealDB. Nodes with saved positions load as `pinned: true`.

## Verification

1. `dx build 2>&1 | grep -v 'INFO'` — zero warnings
2. `dx serve --platform desktop` — app launches
3. Add a location via file picker — node appears, simulation places it
4. Click a directory node — children appear, simulation fans them out naturally
5. Drag a node — it pins, other nodes adjust around it
6. Draw an edge between two nodes — bezier curve appears
7. Notification toast appears on invalid action (e.g., trying to expand a file)
8. Engine code compiles with `safe_mode` on (default) — file ops are no-ops
