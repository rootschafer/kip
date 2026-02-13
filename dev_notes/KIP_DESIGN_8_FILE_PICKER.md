# Kip — Custom File Picker

## Why Not the OS Picker

The native file picker (`rfd` / NSOpenPanel) has several deal-breaking limitations:

1. **No drag-to-workspace**: The user can't drag a file or directory from the OS picker onto the graph. They have to pick, close, and it appears as a node. That's fine for one file, but for 20 locations it's miserable.
2. **No state persistence**: Every time you open the picker, you start from scratch. If you just added `/Users/anders/Photos/Family/Grandma`, and now you want to add `/Users/anders/Photos/Family/Vacations`, you have to navigate the entire path again.
3. **Can't have multiple open**: You get one picker at a time. You can't have one picker browsing your MacBook and another browsing a USB drive simultaneously.
4. **Styling**: It doesn't match the glassmorphic UI at all.

We build our own.

## Column View (like Finder)

The picker uses a **column view** — the default Finder layout on macOS. Each column shows the contents of a directory. Clicking a directory opens its contents in the next column to the right. The view scrolls horizontally as you navigate deeper.

```
┌──────────────────────────────────────────────────────────────────┐
│  /Users/anders                                                   │
│ ┌──────────────┬───────────────┬────────────────┬──────────────┐ │
│ │ Desktop      │ Projects      │ kip            │ src          │ │
│ │ Documents    │ Photos     ▸  │ README.md      │ app.rs       │ │
│ │ Downloads    │ Music         │ Cargo.toml     │ db.rs        │ │
│ │ Movies       │ kip        ▸  │ src         ▸  │ main.rs      │ │
│ │ Music        │ .config       │ dev_notes   ▸  │ engine/   ▸  │ │
│ │ Photos    ▸  │               │ assets      ▸  │ ui/       ▸  │ │
│ │ Projects  ▸  │               │                │ models/   ▸  │ │
│ └──────────────┴───────────────┴────────────────┴──────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

### Column Details

- Each column shows one directory's contents, sorted alphabetically with directories first
- Directories have a `▸` chevron on the right edge
- Clicking a directory highlights it and opens its contents in the next column
- Clicking a file highlights it (no new column opens)
- The column area scrolls horizontally to accommodate deep navigation
- Each column has a fixed width (~180px) but the pane itself is resizable

### File/Directory Info

When a file is selected (highlighted), show basic metadata at the bottom of the picker pane:
- Name, size, modified date
- For images: thumbnail preview (future)

## Pane Behavior

### Opening

The file picker opens as a **right-aligned pane** overlaying the workspace. It doesn't replace the graph — it floats on top of it.

The user can open the picker by:
- Clicking the "+" button and then clicking a machine/drive (same flow as before, but now opens our picker instead of NSOpenPanel)
- A dedicated "Browse" button somewhere in the toolbar (always available)

### Positioning & Resizing

- Default position: right-aligned, vertically centered
- The pane is **draggable** — click and drag the title bar to move it anywhere
- The pane is **resizable** — drag the edges (left, top, bottom, or corners) to resize
- Minimum size: ~300×200px
- Pane position and size are remembered per-session (not persisted to DB — resets on app restart)

### Glassmorphic Styling

The picker pane matches the app's visual language:
- `backdrop-filter: blur(40px)` background
- `rgba(30, 33, 42, 0.85)` background color
- Subtle border: `rgba(255, 255, 255, 0.08)`
- Box shadow for depth
- Same Inter font, same color variables
- Column dividers as subtle vertical lines

## Drag-to-Workspace

This is the killer feature. The user can **drag a file or directory from the picker directly onto a machine/drive container in the graph** to create a location node.

### How It Works

1. User navigates to a file or directory in the picker
2. Mousedown on the item starts a drag
3. A ghost element follows the cursor showing the item name
4. Dragging over a container in the graph highlights it (drop target)
5. Dropping onto a container creates a `location` record in SurrealDB with that container as the owner and the full path as the location path
6. The node appears in the graph immediately

### Multi-Drag

If the user has selected multiple items (shift+click or cmd+click in the picker), dragging any selected item drags all of them. Dropping creates one node per item, all in the same container.

### Drop Validation

- Can only drop onto connected containers (not offline machines/drives)
- Can only drop items that belong to the container's filesystem (can't drag a MacBook path onto a USB drive container — the path wouldn't exist there)
- Invalid drops show a "not allowed" cursor

## Persistent Panes & Minimizing

### Multiple Panes

The user can have **multiple picker panes** open simultaneously. Each pane browses a different location. This is critical for workflows like:
- Browse MacBook ~/Photos in one pane
- Browse USB drive /Volumes/BACKUP in another pane
- Drag from one, drop onto the other's container

### Minimizing

When the user clicks outside a picker pane (on the workspace), the pane **minimizes to the bottom** of the screen. It doesn't close — it preserves its full navigation state.

Minimized panes appear as small labeled tabs at the bottom edge of the workspace:
```
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│                        (workspace)                               │
│                                                                  │
│                                                                  │
├──────────┬──────────┬────────────────────────────────────────────┤
│ ~/Photos │ /BACKUP  │                                            │
└──────────┴──────────┘                                            │
```

- Each minimized tab shows a short path label (last 1-2 path components of the current directory)
- Clicking a minimized tab restores the pane to its previous position and size
- The pane opens exactly where the user left it, scrolled to the same column position
- Right-clicking a minimized tab closes it permanently

### Why This Matters

Imagine adding 15 directories from your Photos library to Kip. Without persistent panes, you'd navigate to `~/Photos/Family` 15 times. With persistent panes, you navigate there once, drag out `Grandma`, `Vacations`, `Holidays`, `Wedding`, one after another. The pane stays right where you were.

## State Management

Each picker pane maintains:
- **Current path**: The deepest selected directory
- **Column state**: The full column hierarchy from root to current
- **Scroll position**: Horizontal scroll offset
- **Selected items**: Currently highlighted files/directories
- **Pane geometry**: Position (x, y) and size (width, height)
- **Minimized flag**: Whether the pane is minimized or visible
- **Root context**: Which machine/drive this picker is browsing (needed for drop validation)

This state lives in Dioxus signals — it's per-session, not persisted to SurrealDB.

## Keyboard Navigation (future)

- Arrow keys to navigate within a column
- Right arrow to enter a directory
- Left arrow to go back
- Enter to select/confirm
- Escape to close/minimize the pane
- Cmd+A to select all in current column

## Integration with the "+" Button

The current flow (click "+" → pick machine/drive → native picker) changes to:

1. Click "+" → add panel appears (same as now)
2. Click a machine/drive → **custom file picker pane opens**, rooted at that machine's home directory or drive's mount point
3. User navigates in the picker and drags items onto the graph
4. Or: user selects items in the picker and clicks an "Add" button at the bottom of the pane

The add panel itself stays the same (list of machines/drives + "Add remote machine"). Only the picker that opens after selecting a target changes from NSOpenPanel to our custom picker.

## Implementation Notes

- The picker reads the filesystem via `std::fs::read_dir()` for local paths
- For remote machines (future): the picker would use SSH/SFTP to list directories
- File icons: use file extension to determine type, show a simple icon or color-coded dot
- Hidden files (dotfiles): hidden by default, toggle to show
- Symlinks: follow them but show a visual indicator
- The picker component lives in `src/ui/file_picker.rs`
- Drop handling on the graph side requires adding `ondragover` / `ondrop` handlers to container elements in `graph.rs`
