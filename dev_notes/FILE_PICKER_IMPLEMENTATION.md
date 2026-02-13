# File Picker Implementation Guide

## Status: COMPLETE ✅

### What's Done ✅
- `src/ui/file_picker.rs` — CREATED and complete. Contains:
  - `PickerManager` (context-provided shared state, wraps `Signal<Vec<PickerPaneData>>`)
  - `FilePickerLayer` component (renders open panes + minimized tab bar)
  - `PickerPaneView` component (column-view file browser with dir navigation)
  - `add_location_from_picker()` DB action (creates location record)
  - `read_dir_sorted()` async dir reader (uses `tokio::task::spawn_blocking`)
  - Helper types: `FsEntry`, `PickerColumn`, `PickerPaneData`

### What's NOT Done Yet
1. **Drag-to-workspace** — Deferred to next pass. Current impl uses "Add to workspace" button.
2. **Multiple pane positioning** — Currently all panes stack at same position. Future: draggable title bars with offsets.
3. **Keyboard navigation** — Arrow keys, Enter, Escape support (future enhancement)
4. **File metadata panel** — Show modified date at bottom when file selected (future)
5. **Persistent pane positions** — Remember x/y/width/height per session (future)

---

## Integration Steps (ALL COMPLETE ✅)

### Step 1: Update `src/ui/mod.rs` ✅
DONE: Added `pub mod file_picker;`

### Step 2: Update `src/app.rs` ✅
DONE: Added imports, context provider, and FilePickerLayer render call.

### Step 3: Update `src/ui/graph.rs` ✅
DONE: Imported PickerManager, got context in MappingGraph, replaced rfd picker with picker.open() call. Removed pick_and_add and add_location functions (now dead code).

### Step 4: Add CSS to `assets/main.css` ✅
DONE: Added full glassmorphic picker styles at end of file.

### Step 5: Build and Test ✅
DONE: `dx build` succeeds. App builds to `/Users/anders/kip/target/dx/kip/debug/macos/Kip.app`

---

## Running and Testing
- Click "+" button → pick a machine/drive → custom picker should open
- Navigate directories by clicking folders
- Select a file or directory → click "Add to workspace" → node appears in graph
- Click minimize (−) → pane minimizes to bottom tab
- Click tab → pane restores
- Right-click tab → pane closes

---

## Architecture Notes

### Signal Flow
```
App
├── provides PickerManager context (Signal<Vec<PickerPaneData>>)
├── MappingGraph
│   ├── reads PickerManager to open new panes
│   └── add panel onclick → picker.open(container_id, name, root_path)
├── FilePickerLayer
│   ├── reads PickerManager to render panes
│   ├── PickerPaneView (one per open pane)
│   │   ├── use_effect: loads root dir on mount (async)
│   │   ├── entry onclick: truncates columns, loads next dir (async)
│   │   └── "Add" button: calls add_location_from_picker (async DB write)
│   └── Minimized tab bar
└── ReviewQueue
```

### Key Patterns
- **All state in one Signal**: `PickerManager` wraps `Signal<Vec<PickerPaneData>>`. All mutations go through `.0.write()`.
- **Async dir reads**: `read_dir_sorted()` uses `tokio::task::spawn_blocking` because `std::fs::read_dir` is blocking. NEVER use blocking FS ops on the Dioxus async runtime.
- **Column navigation**: Clicking a dir entry does TWO things: (1) truncates columns after current column, sets selection; (2) async reads the dir and pushes a new column. The write guard MUST be dropped between steps 1 and 2 because of the `.await`.
- **Pane reuse**: If a pane for the same container_id already exists, `open()` just restores it instead of creating a new one.
- **Hidden files**: Toggle via ".*" button in title bar. Clears columns (triggers reload via use_effect).

### DB Action (add_location_from_picker)
Identical to the existing `add_location` in graph.rs. Creates a `location` record with:
- `{table}: $container` (where table is "machine" or "drive")
- `path: $path`
- `available: true`
- `created_at: time::now()`

The `type::record()` call requires the table name as a query literal, NOT a bind parameter (SurrealDB 3.0 gotcha).

---

## Future Enhancements (NOT for this pass)
1. **Drag-to-workspace**: Add HTML5 drag events (`ondragstart` on picker entries, `ondragover`/`ondrop` on graph containers). Ghost element follows cursor.
2. **Multiple pane positioning**: Currently all panes stack at same position. Add offset or allow dragging title bar.
3. **Keyboard navigation**: Arrow keys, Enter, Escape.
4. **File metadata panel**: Show size, modified date at bottom when file selected.
5. **Persistent pane positions**: Remember x/y/width/height per session.

---

## Critical Gotchas (READ THESE)
- **NEVER use `cargo build`** — always `dx build` or `dxbquiet`
- **SurrealDB `type::record()` table name** must be a query string literal, NOT a bind parameter
- **Dioxus RSX**: no inline if/else as direct text content — extract to a `let` variable first
- **Signal write guards**: drop them before any `.await` call
- **`spawn()`** works from event handlers in Dioxus 0.7
- **`use_context::<T>()`** can be called in event handler closures in Dioxus 0.7 (captures the scope)
- **All `DEFINE` statements** in SurrealDB need `OVERWRITE` keyword for idempotency
- **`.bind()` needs owned values** (String, not &String)
