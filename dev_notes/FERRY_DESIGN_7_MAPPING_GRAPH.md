# Ferry — Mapping Graph UI

## The Problem with "New Intent"

The current intent creation flow — "type a source path, type a dest path, click go" — has two problems:

1. **Nobody wants to type paths.** File pickers are marginally better but still friction-heavy for bulk setup.
2. **It's backwards.** The user's mental model isn't "copy `/Users/anders/projects` to `/Volumes/SOMETHING/projects`." It's **"my projects should exist on both my MacBook and SOMETHING."** The directionality is incidental — what matters is the mapping.

Ferry should match that mental model.

## The Graph

The core UI is a **2D mapping graph**. Nodes are locations (directories or file paths on a Machine/Drive). Edges are relationships between them — Ferry's job is to **resolve** every edge.

```
┌──────────────────┐         ┌──────────────────┐
│  MacBook         │         │  SOMETHING       │
│  ~/projects      │────────▶│  /projects       │
│                  │         │                  │
│  ~/photos        │◀───────▶│  /photos         │
│                  │         │                  │
│  ~/Documents     │────┐    └──────────────────┘
└──────────────────┘    │
                        │    ┌──────────────────┐
                        └───▶│  derver          │
                             │  /backup/docs    │
                             └──────────────────┘
```

### Nodes

A node is a **location**: a path on a Machine or Drive. Users add nodes by:
- Clicking "Add" and picking a directory/file via native file picker (for local/mounted paths)
- Typing a path (for remote machines, or power users)
- Dragging a folder from Finder onto the graph

Nodes are grouped visually by their Machine/Drive. Each Machine/Drive is a labeled container holding its location nodes.

### Edges

An edge connects two nodes and says "these should be in sync." Edges have two phases:

**Phase 1 — Directional (arrow)**
When a user first draws an edge from node A to node B, it's a one-way arrow: "copy A → B." This is the initial transfer. The edge displays as an arrow with transfer progress.

**Phase 2 — Bidirectional (line)**
Once the first full sync/resolution completes (all files copied, no unresolved conflicts), the edge becomes bidirectional. Now changes on either side propagate to the other. The edge displays as a plain line (no arrow).

A user can also explicitly create a one-way edge that stays directional (for backup-only scenarios where the destination is read-only/archival). This is a toggle on the edge: "Keep one-way" vs "Sync both ways after first transfer."

### Edge States

| State | Visual | Meaning |
|-------|--------|---------|
| `idle` | Thin gray line/arrow | Nothing happening |
| `syncing` | Animated blue line/arrow | Scanning or transferring |
| `resolved` | Solid green line | Fully synced, no conflicts |
| `conflict` | Orange line with badge | Has unresolved sync conflicts |
| `waiting` | Dashed line | One side is offline/disconnected |
| `error` | Red line with badge | Transfer errors need review |

## Conflict Resolution

When a bidirectional edge detects changes on both sides, it becomes a conflict. The user is shown resolution UI based on what conflicted:

### Text Files
Full diff view (unified or side-by-side). The user sees exactly what changed on each side and can:
- Keep left / Keep right / Merge manually
- For code files: syntax-highlighted diff

### Binary Files (images, models, etc.)
Side-by-side preview with metadata comparison:
```
┌─────────────────────────────────────────────┐
│ ⚠ CONFLICT: render_final.png                │
│                                             │
│  MacBook            │  SOMETHING            │
│  [image preview]    │  [image preview]      │
│  2.4 MB             │  1.8 MB               │
│  Modified today     │  Modified Jan 28      │
│  2048×1536          │  1920×1080            │
│                                             │
│  [Keep Left] [Keep Right] [Keep Both] [Skip]│
└─────────────────────────────────────────────┘
```

### Directory-Level Conflicts
When a mapping involves directories that have diverged structurally (files added/removed/moved on both sides), show a **file tree diff**:

```
┌─────────────────────────────────────────────┐
│ ⚠ Directory conflict: /projects             │
│                                             │
│  MacBook               SOMETHING            │
│  ├── src/              ├── src/             │
│  │   ├── main.rs       │   ├── main.rs  ~  │  ← modified both sides
│  │   ├── lib.rs    +   │   │                │  ← added on left only
│  │   └── util.rs       │   ├── util.rs     │
│  │                     │   └── old.rs   +  │  ← added on right only
│  └── README.md         └── README.md       │
│                                             │
│  3 differences                              │
│  [Merge All →] [Merge All ←] [Review Each] │
└─────────────────────────────────────────────┘
```

The "Review Each" option walks through each conflicted file individually.

## Interaction Model

### Adding a Mapping
1. User sees the graph with existing Machine/Drive containers
2. Clicks "Add Location" on a container (or drags folder from Finder)
3. Node appears in the container
4. User drags from one node to another to create an edge
5. Edge starts as directional (arrow from source to dest)
6. Ferry begins scanning and transferring

### Removing a Mapping
- Click an edge → delete. This does NOT delete files — it just means Ferry stops caring about keeping them in sync.
- Click a node → delete. Removes the node and all its edges. Again, files are untouched.

### Discovering Machines/Drives
- Local machine appears automatically
- Mounted drives appear automatically via DiskArbitration
- Remote machines are added manually (Phase 2): name + SSH config

### Graph Layout — Force-Directed with Columnar Seed

The graph uses a **force-directed layout** that starts columnar and transitions to freeform as complexity grows.

**At small scale (2-3 machines, <10 nodes):**
Machines/Drives appear as columns. Nodes are listed vertically within each column. This is clean, obvious, and familiar — looks like a standard mapping diagram. No algorithm needed; just column placement.

**At larger scale (5+ machines, 20+ nodes):**
The layout transitions to force-directed. The forces:
- **Edge attraction**: Connected nodes pull toward each other
- **Crossing minimization**: The layout continuously repositions to reduce edge crossings
- **Container cohesion**: Nodes on the same Machine/Drive stay grouped (soft constraint)
- **Same-machine attraction**: Nodes sharing a Machine/Drive (same color) attract each other, keeping like-colored clusters together even as the layout goes freeform
- **Repulsion**: Unconnected clusters spread apart to avoid clutter

**Initial node placement — path similarity seeding:**
When a new node is added, it's placed near existing nodes with similar OS paths. `/Users/anders/projects/web` appears next to `/Users/anders/projects/api`, not at a random position. This means nodes that are likely to share connections start close together, so the user rarely has to search far to draw an edge.

**User overrides:**
Users can drag any node or container to reposition it. Pinned positions are respected — the force algorithm works around them. This lets users impose their own mental organization when the algorithm's choice doesn't match.

**Machine/Drive color-coding:**
Each Machine/Drive gets a **distinct color** that carries through:
- Container border and header background
- Node background tint
- Edge endpoints (edges are drawn as gradients between the two machine colors)

This makes it instantly visually parseable — you can glance at any edge and know "blue machine → orange drive" without reading labels. Colors are assigned automatically from a palette designed for contrast against the dark background.

**Layout persistence:**
Node and container positions are stored in SurrealDB. The graph looks the same every time you open Ferry. Force simulation only runs when nodes are added/removed or the user drags something — it's not constantly animating.

**Edge routing:**
Edges route as bezier curves between containers. When many edges connect the same two machines, they fan out slightly to remain individually selectable.

## Data Model Changes

The existing `intent` table evolves. An "intent" is now an edge in the graph:

```surql
-- intent gains directionality tracking
DEFINE FIELD bidirectional ON intent TYPE bool DEFAULT false;
DEFINE FIELD initial_sync_complete ON intent TYPE bool DEFAULT false;
-- When initial_sync_complete flips to true AND bidirectional is true,
-- the edge becomes a two-way sync.

-- For bidirectional edges, "source" just means "which side was populated first."
-- Both sides are equal after initial sync.
```

No other schema changes needed — the existing Location → Intent → TransferJob pipeline still works. Bidirectional sync just means when a change is detected on either side, Ferry creates jobs to propagate it to the other.

## What This Replaces

The "New Intent" form (type source path, type dest path, click create) goes away entirely. The graph IS the intent creation UI. Drawing an edge creates an intent. The form was a scaffolding step for MVP — the graph is the real thing.

## Phasing

### MVP (now)
- Keep the current text-input "New Intent" form as a stopgap
- It works, it tests the engine, it's not the final UI

### Next Priority (before other Phase 2 work)
- Graph UI with Machine/Drive containers and location nodes
- Drag-to-connect edge creation
- Directional → bidirectional edge lifecycle
- Native file picker for adding nodes (not typing paths)
- Edge state visualization (syncing, resolved, conflict, waiting)

### Phase 2+
- Conflict resolution UI (diffs, side-by-side preview, tree diff)
- Finder drag-and-drop onto graph
- Remote machine containers
- Graph layout persistence
