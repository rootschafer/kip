# Kip — Mapping Graph UI

## The Core Idea

The graph IS the app. Kip's primary interface is a 2D workspace where the user sees every location, every relationship, and every problem at a glance. The mental model is spatial: "my projects live here and here, connected by this line." Drawing a line between two nodes creates an intent. That's it.

The graph replaces the old "type a source path, type a dest path" flow entirely.

## Visual Design: iOS Glassmorphism

The entire UI uses an iOS-inspired glassmorphic aesthetic:

- **Dark background** with glass-effect containers (`backdrop-filter: blur(24px)`, `rgba` backgrounds)
- **Frosted glass panels** for containers, toolbars, and overlays
- **Subtle borders** with `rgba(255,255,255,0.08)` edges
- **CSS variables** for theming: `--bg`, `--glass`, `--glass-border`, `--glass-hover`, `--text`, `--text-dim`, `--accent`
- **Inter font** (or system sans-serif fallback) for clean typography
- **Accent color**: `#4a9eff` (blue) used for the "+" button, selection highlights, lasso outline, and active edges
- **Minimal chrome** — Kip gets out of the way. The graph workspace takes up most of the screen.

## Architecture: HTML + SVG Overlay

SVG doesn't support `backdrop-filter`, so the graph uses a hybrid rendering approach:

- **HTML layer**: Machine/drive containers and location nodes are rendered as `<div>` elements with glass effects. This layer handles all the visual styling.
- **SVG overlay**: Edges (bezier curves between nodes), the rubber-band line during edge creation, and the lasso selection rectangle are rendered as SVG elements positioned on top of the HTML layer.

Both layers are positioned absolutely within a shared coordinate space. The HTML layer handles click/drag events and delegates to the SVG layer for visual feedback.

## Containers (Machines & Drives)

Machines and drives are rendered as labeled glass containers. Each gets a **distinct palette color** that carries through:
- Container header dot
- Node backgrounds (tinted)
- Edge endpoints (edges gradient between source and dest colors)

### Container States

| State | Visual |
|-------|--------|
| Local machine | Color dot + name + "local" label |
| Remote machine (online) | Color dot + name + "remote" label |
| Remote machine (offline) | Color dot + name + "offline" label, dimmed container |
| Drive (connected) | Color dot + name + "drive" label |
| Drive (disconnected) | Dimmed container, "offline" label |

Containers appear automatically:
- **Local machine**: Always present on launch
- **Mounted drives**: Detected via DiskArbitration, appear/disappear as drives connect
- **Remote machines**: Added manually via the add panel

## Nodes (Locations)

A node is a **location**: a path on a machine or drive. Nodes live inside their parent container.

### Adding Nodes

**Primary method — drag from file picker**:
1. Click the blue **"+"** button in the top-right toolbar
2. A glass overlay panel appears listing all available machines and drives
3. Click a machine or drive — the **custom file picker** pane opens (column view, see `KIP_DESIGN_8_FILE_PICKER.md`)
4. Navigate to the file or directory you want
5. **Drag** it from the picker onto a container in the graph — a location node is created instantly

**Secondary method — "Add" button in picker**: Select files/dirs in the picker and click an "Add" button at the bottom of the pane.

The "+" panel also has an **"+ Add remote machine"** option at the bottom (below a divider) that opens an inline form with fields for Name, Hostname, and SSH User.

**Key UX decisions**:
- The file picker is our own custom component (not the OS native picker). This enables drag-to-workspace, persistent navigation state, and multiple simultaneous panes.
- When clicking outside a picker pane, it **minimizes** to the bottom of the screen (doesn't close). Clicking the minimized tab restores it exactly where you left off. This preserves navigation state so you don't re-navigate to the same deep folder over and over.
- Multiple picker panes can be open at once (e.g., one browsing MacBook, one browsing a USB drive).

### Path Containment (Set Theory)

When multiple nodes exist within the same container, Kip detects hierarchical relationships:

- `/Users/anders/projects` **contains** `/Users/anders/projects/kip`
- `/Users/anders/projects/kip` **contains** `/Users/anders/projects/kip/src`
- `/Users/anders/projects` does **NOT** contain `/Users/anders/projects-old` (trailing-slash normalization prevents false prefix matches)

**Visual treatment**:
- Contained nodes are indented (12px per depth level) within their container
- A subtle left-border indicator marks nested nodes
- Depth is computed dynamically: 0 = top-level, 1 = contained by one other path, 2 = contained by two, etc.

This lets users see at a glance that `/projects/kip/src` is inside `/projects/kip` which is inside `/projects`.

### Node Shapes: Circles vs. Pills

Nodes have two shapes depending on what they represent:

- **Files** render as **pills** (rounded rectangles) — the current shape. They are leaf nodes with no children.
- **Directories** render as **circles**. They can be expanded to show their contents.
- **Groups** also render as **circles**. They behave like directories conceptually (contain other nodes).

This visual distinction makes it immediately obvious what's a navigable container vs. a leaf.

### Directory/Group Circle Interaction (NOT YET IMPLEMENTED)

Circles have a two-level expansion model:

**Click once — Orbit view**: The circle's direct children (level 1) appear as smaller nodes arranged in a ring *around* the circle. The parent circle stays in place and the children fan out around it, connected by short edges. This gives a quick preview without navigating.

**Click again — Enter**: The view transitions to show the contents *inside* the circle (or the circle expands to become a container). Now only the direct children are visible as full nodes, and the parent becomes the current context. A breadcrumb or back button allows navigating back up.

This model is recursive — a directory circle inside another directory circle follows the same click-once/click-again pattern.

### Node Display

Each node shows:
- A shortened path label (last 2 path components, e.g., `.../projects/kip`)
- For pills: a drag handle on the right edge (for creating edges)
- For circles: the label is centered inside the circle

## Edges (Intents)

An edge connects two nodes and says "these should be in sync." Drawing an edge creates an intent in SurrealDB.

### Creating Edges

1. **Mousedown** on a node (without Shift) starts a drag
2. A dashed blue rubber-band line follows the cursor
3. **Mouseup** on another node creates the edge (and the underlying intent)
4. **Mouseup** on empty space cancels

### Edge Phases

**Phase 1 — Directional (arrow)**
When first created, an edge is a one-way arrow: "copy A → B." It displays as an arrow with transfer progress.

**Phase 2 — Bidirectional (line)**
Once initial sync completes (all files copied, no conflicts), the edge can become bidirectional. Changes on either side propagate to the other. Displays as a plain line (no arrow).

Users can explicitly lock an edge to one-way for backup-only scenarios.

### Edge States & Colors

| State | Color | Width | Meaning |
|-------|-------|-------|---------|
| `idle` | Gray `#555` | 2px | Nothing happening |
| `scanning` | Blue `#4a9eff` | 3px | Enumerating files |
| `transferring` | Blue `#4a9eff` | 3px | Actively copying |
| `complete` | Green `#3fb950` | 2px | Fully synced |
| `needs_review` | Orange `#d29922` | 2px | Has unresolved issues |
| `failed` | Red `#f85149` | 2px | Transfer errors |
| `waiting` | Dashed gray | 2px | One side offline |

Edges render as cubic bezier curves (not straight lines). When many edges connect the same two containers, they fan out slightly to remain individually clickable.

## Selection System

### Shift+Click (Individual Select)

- **Shift+click** on a node toggles its selection state
- Selected nodes get a blue border glow and highlighted background
- Multiple nodes can be selected across different containers
- **Click on empty space** (without Shift) deselects all

### Shift+Drag Lasso (Area Select)

- **Shift+drag** on empty space creates a lasso selection rectangle
- The rectangle renders as a dashed blue border with subtle blue fill
- On mouse release, all nodes whose center falls within the rectangle are selected
- Lasso adds to existing selection (doesn't replace)

### Selection State

Selection is tracked as a `HashSet<String>` of node IDs (e.g., `"location:abc123"`).

## Grouping (NOT YET IMPLEMENTED)

Grouping is the next major feature after selection works. It reduces visual noise and creates a layered, collapsible system.

### What Grouping Does

When the user selects multiple nodes and groups them:

1. The selected nodes collapse into a single **group node** in the workspace
2. The group shows a summary label (e.g., "3 locations" or a user-provided name)
3. All edges connected to any member node now connect to the group instead
4. Double-clicking a group expands it, revealing its members (like zooming into a folder)
5. Groups can be nested — a group can contain other groups

### Why Grouping Matters

Consider a user with 20 directories on their MacBook all syncing to a backup drive. Without grouping, that's 20 nodes and 20+ edges cluttering the graph. With grouping:

- Select all the project directories → group them as "Projects"
- Select all the media directories → group them as "Media"
- Now the graph shows 2 clean groups instead of 20 nodes
- Each group has a single edge to the backup drive
- The user can expand a group to inspect or modify individual mappings

### Group Data Model

Groups are stored in SurrealDB and survive restarts:

```surql
DEFINE TABLE node_group SCHEMAFULL;
DEFINE FIELD name ON node_group TYPE string;
DEFINE FIELD members ON node_group TYPE array<record<location>>;
DEFINE FIELD parent_group ON node_group TYPE option<record<node_group>>;
DEFINE FIELD collapsed ON node_group TYPE bool DEFAULT true;
DEFINE FIELD created_at ON node_group TYPE datetime;
```

### Group Visual Treatment

- A group renders as a **circle** (same as directories — circles = things with children)
- When collapsed: shows group name + member count + aggregate status indicator inside the circle
- **Click once** (orbit view): member nodes fan out around the circle in a ring
- **Click again** (enter): transitions to show members as full nodes inside an expanded container
- Groups can be dragged, selected, and connected just like individual nodes

### Edge Merging When Grouping

When nodes are grouped, their outgoing/incoming edges merge:

- If nodes A, B, C all connect to node D, and A/B/C are grouped → one edge from the group to D
- The merged edge represents the combined status (worst status wins: if any sub-edge has errors, the merged edge shows errors)
- Expanding the group reveals the individual edges again

## Central "Output" Node (NOT YET IMPLEMENTED)

In the center of the workspace, there is a special circular node labeled **"Output"**. This is the merge/collection point for the entire workspace.

### What Output Does

- Any node or group connected to Output declares: "this is a final destination"
- Output acts as a visual anchor — it's always centered and slightly larger than regular nodes
- Edges pointing TO Output mean "send files here"
- Edges pointing FROM Output mean "distribute from here"

### Why Output Exists

The Output node gives the workspace a focal point. Instead of a web of edges going everywhere, the user can organize their graph radially:

- Sources on the outer ring
- Output in the center
- Everything flows inward (backup) or outward (distribution)

This maps to common use cases:
- **Backup**: Multiple source directories → Output (a backup drive or server)
- **Distribution**: Output (a master copy) → multiple destinations
- **Migration**: Old machine sources → Output → new machine destinations

### Output Visual Treatment

- Circular shape (not rectangular like location nodes)
- Slightly larger than a regular node
- Always positioned at the center of the workspace (or user-draggable)
- Distinct styling: perhaps a subtle radial gradient or pulsing glow when active
- Label: "Output" by default, user can rename

### Output Data Model

Output is a special location (or virtual node) — implementation TBD. It might be:
- A virtual node with no real filesystem path (just a merge point in the UI)
- Or a real location that the user designates as the collection point

## Status & Error Indicators

### Global Status Indicator

In the top-left of the toolbar (next to the "+" button on the right), there's a status indicator dot:

| State | Visual | Meaning |
|-------|--------|---------|
| All clear | Green dot, no label | No pending review items |
| Issues | Red dot + count + "issue(s)" label | N review items need attention |

The status indicator is always visible, even when everything is fine (green dot = reassurance). It queries `review_item WHERE resolution IS NONE` on every refresh.

### Per-Node / Per-Group Error Indicators (NOT YET IMPLEMENTED)

Each node and group has an optional error badge at its **top-left corner** (sitting on the rounded corner edge of the node's bounding box):

| State | Visual |
|-------|--------|
| No issues | No badge (clean) |
| Warning(s) | Small yellow circle with count |
| Error(s) | Small red circle with count |

The badge shows the count of unresolved review items associated with that node's location (or, for groups, the aggregate of all member nodes).

Clicking the badge scrolls the review queue to show the relevant items.

### Error Handling Philosophy

**Errors NEVER appear as text in the UI.** If something goes wrong during data loading, graph rendering, or any background operation:

1. Log it with `error!()`, `warn!()`, or `info!()` via the `tracing` crate (re-exported from `dioxus::prelude::*`)
2. Render the UI in a degraded-but-functional state (empty graph, missing data, etc.)
3. Only surface errors to the user when they need to **take action** (review items in the queue)

This is a hard rule. The user should never see "Error loading graph" or a stack trace. If the graph can't load, it's empty. If a query fails, the relevant section is missing. Errors go to `kip.log`.

## Graph Layout

### Current: Static Columnar

Containers are laid out in a horizontal row with fixed spacing:
- `CONTAINER_WIDTH = 200px`
- `CONTAINER_GAP = 32px`
- `GRAPH_PADDING = 24px`

Nodes are stacked vertically within each container with `NODE_HEIGHT = 36px` and `NODE_GAP = 4px`. Nested nodes are indented by `INDENT_PX = 12px` per depth level.

### Future: Force-Directed with Columnar Seed

At small scale (2-3 machines, <10 nodes): columnar layout as described above. Clean, obvious, familiar.

At larger scale (5+ machines, 20+ nodes): transitions to force-directed layout with these forces:

- **Edge attraction**: Connected nodes pull toward each other
- **Crossing minimization**: Layout repositions to reduce edge crossings
- **Container cohesion**: Nodes on the same machine/drive stay grouped
- **Same-machine attraction**: Same-color nodes attract each other
- **Repulsion**: Unconnected clusters spread apart

**Path similarity seeding**: New nodes are placed near existing nodes with similar paths. `/Users/anders/projects/web` appears next to `/Users/anders/projects/api`.

**User overrides**: Users can drag any node or container. Pinned positions are respected — the force algorithm works around them.

**Layout persistence**: Positions stored in SurrealDB. The graph looks the same every time you open Kip.

## Interaction Summary

| Action | Result |
|--------|--------|
| Click "+" button | Open add panel (list of machines/drives) |
| Click machine/drive in add panel | Open custom file picker pane for that target |
| Drag file/dir from picker onto container | Create location node in that container |
| Click "+ Add remote machine" | Show inline form for name/host/user |
| Mousedown on pill node (no modifier) | Start edge creation drag |
| Mouseup on another node | Create intent (edge) between them |
| Mouseup on empty space | Cancel edge creation |
| Click circle node once | Orbit view — children fan out around it |
| Click circle node again | Enter — show children as full nodes inside |
| Shift+click on node | Toggle node selection |
| Shift+drag on empty space | Lasso select (area selection) |
| Click on empty space (no modifier) | Deselect all |
| Click outside picker pane | Minimize pane to bottom tab |
| Click minimized picker tab | Restore pane to previous position |
| Click edge | (future) Select edge, show details |
| Right-click node | (future) Context menu: delete, rename, etc. |

## Data Flow

1. `MappingGraph` component receives `refresh_tick` (incremented when data changes) and `on_changed` callback
2. `use_resource` loads graph data from SurrealDB whenever `refresh_tick` changes
3. Data loading queries: machines, drives, locations, intents, review_item count
4. Results are transformed into view types: `ContainerView`, `NodeView`, `EdgeView`
5. Containment depth is computed for nodes within the same container
6. HTML renders containers and nodes; SVG overlay renders edges and interaction visuals
7. User actions (add location, create edge, etc.) write to SurrealDB and call `on_changed` to trigger refresh

## Technical Notes

- `ContainerView` includes `mount_point: Option<String>` to set file picker root directory
- `NodeView` includes `depth: usize` for containment-based indentation
- `EdgeView` maps directly from intent records (one edge per intent, using first destination)
- `rid_string()` helper converts `RecordId` to `"table:key"` format (RecordId has no Display impl)
- `type::record()` table name must be a query literal, not a bind parameter — use `format!()` to interpolate
- Review count uses `SELECT count() AS count FROM review_item WHERE resolution IS NONE GROUP ALL`

## What's Implemented vs. Planned

### Implemented
- [x] Glass containers for machines and drives (USER VERIFIED)
- [x] Location nodes with path labels inside containers (USER VERIFIED)
- [x] Path containment detection and visual nesting (USER VERIFIED)
- [x] Edge creation via drag between nodes (bezier curves, colored by status) (USER VERIFIED)
- [x] "+" button → add panel → file picker flow (no intermediate step) (USER VERIFIED)
- [x] Remote machine creation form (USER VERIFIED)
- [x] Shift+click multi-select (USER VERIFIED)
- [x] Shift+drag lasso selection (USER VERIFIED)
- [x] Global status indicator (green/red dot with review count) (USER VERIFIED)
- [x] Glassmorphic CSS throughout (USER VERIFIED)
- [x] Errors logged to tracing, never shown in UI (USER VERIFIED)
- [√] Store-based PickerManager for reactive state management (AI CONFIRMED)

### Not Yet Implemented
- [~] Circular nodes for directories and groups (pills for files) - PARTIALLY IMPLEMENTED
  - **Details**: See `Phase1/Phase1.1_Directory_Expansion_Implementation.md`, `Phase1/Phase1.1_Directory_Expansion_and_File_Picker.md`
- [~] Directory expansion: orbit view (children fanned out around parent) - PARTIALLY IMPLEMENTED
  - **Details**: See `Phase1/Phase1.1_Directory_Expansion_Implementation.md`, `Phase1/Phase1.1_Directory_Expansion_and_File_Picker.md`
- [~] Directory expansion: enter view (workspace shows only direct children) - PARTIALLY IMPLEMENTED
  - **Details**: See `Phase1/Phase1.1_Directory_Expansion_Implementation.md`, `Phase1/Phase1.1_Directory_Expansion_and_File_Picker.md`
- [~] Dynamic node sizing based on total descendant count - PARTIALLY IMPLEMENTED
  - **Details**: See `Phase1/Phase1.1_Directory_Expansion_Implementation.md`, `Phase1/Phase1.1_Directory_Expansion_and_File_Picker.md`
- [ ] Custom file picker (column view, drag-to-workspace, persistent panes)
- [ ] Grouping (select nodes → group → collapse/expand)
  - **Details**: See `Phase1/Phase1.2_Node_Grouping_Implementation.md`
- [ ] Central "Output" node
- [ ] Per-node/group error badges
- [ ] Force-directed layout
  - **Details**: See `Phase1/Phase1.3_Force_Directed_Layout_Implementation.md`
- [ ] Layout persistence (positions saved to DB)
- [ ] Edge click/select/delete
- [ ] Node delete / context menu
- [ ] Edge state animations (pulsing for active transfers)
- [ ] Drag from picker onto graph (drop handler)
- [ ] Color-coded edge gradients (source color → dest color)
