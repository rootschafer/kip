# File Picker Implementation Guide

## Status: IN PROGRESS

### What's Done
- `src/ui/file_picker.rs` — CREATED and complete. Contains:
  - `PickerManager` (context-provided shared state, wraps `Signal<Vec<PickerPaneData>>`)
  - `FilePickerLayer` component (renders open panes + minimized tab bar)
  - `PickerPaneView` component (column-view file browser with dir navigation)
  - `add_location_from_picker()` DB action (creates location record)
  - `read_dir_sorted()` async dir reader (uses `tokio::task::spawn_blocking`)
  - Helper types: `FsEntry`, `PickerColumn`, `PickerPaneData`

### What's NOT Done Yet
1. **CSS** — No picker styles in `assets/main.css` yet. Must add them (see CSS section below).
2. **`src/ui/mod.rs`** — Must add `pub mod file_picker;`
3. **`src/app.rs`** — Must add `PickerManager` context provider and render `FilePickerLayer`
4. **`src/ui/graph.rs`** — Must replace `rfd` picker with opening custom picker pane
5. **Build verification** — Must run `dxbquiet` (alias for `dx build 2>&1 | sed 's/\x1b\[[0-9;]*m//g'`)
6. **Drag-to-workspace** — Deferred to next pass. Current impl uses "Add to workspace" button.

---

## Integration Steps (DO THESE IN ORDER)

### Step 1: Update `src/ui/mod.rs`
Add this line:
```rust
pub mod file_picker;
```

### Step 2: Update `src/app.rs`
Add imports at top:
```rust
use crate::ui::file_picker::{FilePickerLayer, PickerManager};
```

Inside the `App` component, BEFORE the `rsx!` block, add context provider:
```rust
let _picker = use_context_provider(|| PickerManager::new());
```

In the `rsx!` block, add `FilePickerLayer` AFTER `MappingGraph` and BEFORE `ReviewQueue`:
```rust
rsx! {
    document::Stylesheet { href: MAIN_CSS }
    div { class: "app",
        div { class: "header",
            // ... existing header code ...
        }
        MappingGraph { refresh_tick: refresh_tick(), on_changed: on_refresh }
        FilePickerLayer { on_location_added: on_refresh }
        ReviewQueue { refresh_tick: refresh_tick(), on_resolved: on_refresh }
    }
}
```

### Step 3: Update `src/ui/graph.rs`
Import the picker:
```rust
use crate::ui::file_picker::PickerManager;
```

Inside `MappingGraph` component, get the picker context:
```rust
let mut picker = use_context::<PickerManager>();
```

Replace the add panel item `onclick` handler. Currently it calls `pick_and_add` (rfd native picker). Change it to open the custom picker instead. The current handler is around line 459-480. Replace the `spawn(async move { ... pick_and_add ... })` block with:

```rust
move |_| {
    if !connected {
        warn!("cannot add to disconnected target");
        return;
    }
    let root = mount_point.clone().unwrap_or_else(|| "/".to_string());
    picker.open(
        cid.clone(),
        name.clone(),
        std::path::PathBuf::from(root),
    );
    *add_panel.write() = AddPanelState::Closed;
}
```

You can then REMOVE the `pick_and_add` function and the `rfd` import/dependency if desired, or leave them for now.

### Step 4: Add CSS to `assets/main.css`
Append the following styles at the end of the file:

```css
/* ─── File Picker ─── */
.picker-pane {
    position: fixed;
    top: 80px;
    right: 24px;
    width: 720px;
    height: 480px;
    backdrop-filter: blur(40px);
    -webkit-backdrop-filter: blur(40px);
    background: rgba(30, 33, 42, 0.92);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255, 255, 255, 0.05);
    z-index: 300;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: panel-in 0.2s ease-out;
}

.picker-title-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--glass-border);
    flex-shrink: 0;
    cursor: default;
}
.picker-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
}
.picker-title-actions {
    display: flex;
    gap: 4px;
}
.picker-title-actions button {
    width: 28px;
    height: 28px;
    border-radius: 8px;
    background: transparent;
    color: var(--text-dim);
    border: none;
    font-size: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0;
    transition: all 0.15s ease;
}
.picker-title-actions button:hover {
    background: var(--glass-hover);
    color: var(--text);
}
.picker-btn-close:hover {
    background: rgba(248, 113, 113, 0.15) !important;
    color: var(--red) !important;
}
.picker-btn-toggle.active {
    color: var(--accent);
}

.picker-breadcrumb {
    padding: 6px 16px;
    font-size: 11px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: var(--text-muted);
    border-bottom: 1px solid var(--glass-border);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 0;
}

.picker-columns {
    flex: 1;
    display: flex;
    overflow-x: auto;
    overflow-y: hidden;
    min-height: 0;
}

.picker-column {
    min-width: 180px;
    max-width: 220px;
    flex-shrink: 0;
    overflow-y: auto;
    border-right: 1px solid var(--glass-border);
    padding: 4px 0;
}
.picker-column:last-child {
    border-right: none;
}

.picker-entry {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    cursor: pointer;
    transition: background 0.1s ease;
    user-select: none;
}
.picker-entry:hover {
    background: var(--glass-hover);
}
.picker-entry.selected {
    background: rgba(74, 158, 255, 0.12);
}

.entry-icon {
    font-size: 10px;
    flex-shrink: 0;
    width: 14px;
    text-align: center;
}
.entry-icon.dir {
    color: var(--accent);
}
.entry-icon.file {
    color: var(--text-muted);
}

.entry-name {
    font-size: 12px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
}
.picker-entry.selected .entry-name {
    color: #fff;
}

.entry-size {
    font-size: 10px;
    color: var(--text-muted);
    flex-shrink: 0;
    margin-left: auto;
}

.picker-bottom-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-top: 1px solid var(--glass-border);
    flex-shrink: 0;
}
.picker-selected-path {
    flex: 1;
    font-size: 11px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.picker-add-btn {
    flex-shrink: 0;
}

/* ─── Minimized tab bar ─── */
.picker-tab-bar {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    display: flex;
    gap: 2px;
    padding: 0 16px;
    z-index: 250;
    pointer-events: none;
}
.picker-tab {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    background: rgba(30, 33, 42, 0.9);
    border: 1px solid var(--glass-border);
    border-bottom: none;
    border-radius: 10px 10px 0 0;
    cursor: pointer;
    transition: background 0.15s ease;
}
.picker-tab:hover {
    background: rgba(40, 44, 56, 0.95);
}
.picker-tab-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text);
}
.picker-tab-path {
    font-size: 10px;
    color: var(--text-muted);
}
```

### Step 5: Build and Test
```sh
dxbquiet
# or: dx build 2>&1 | sed 's/\x1b\[[0-9;]*m//g'
```

Fix any compilation errors. Common issues:
- Missing imports (check `use` statements)
- Signal borrow issues (make sure `.write()` guard is dropped before `.await`)
- Dioxus RSX syntax (no inline if/else as text — extract to variable first)

### Step 6: Run and Test
```sh
dx serve --platform desktop
```
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
