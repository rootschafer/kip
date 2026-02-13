# File Picker Implementation — Bugs Found & Fixed

## Fixed ✅

### 1. **use_context() called inside event handlers** ✅ FIXED
**Location**: `src/ui/file_picker.rs` lines 203-204, 208-209 (minimized tab bar)

**Problem**:
```rust
onclick: move |_| {
    let mut picker = use_context::<PickerManager>();  // ❌ WRONG!
    picker.restore(id);
}
```

In Dioxus 0.7, you **cannot call hooks** (like `use_context`) inside event handler closures. The context must be captured from the outer scope.

**Fix**: Capture `picker` from outside the closure and reuse it:
```rust
// Applied fix (src/ui/file_picker.rs line 171):
let mut picker = use_context::<PickerManager>();  // Made mutable at component level

// Line 198:
let mut picker_ctx = picker;  // Capture before rsx! block

// Lines 203-204, 206-208:
onclick: move |_| {
    picker_ctx.restore(id);  // Use captured, don't call hooks
},
oncontextmenu: move |e: Event<MouseData>| {
    e.prevent_default();
    picker_ctx.close(id);  // Use captured, don't call hooks
},
```

**Status**: ✅ Applied and compiling

---

### 2. **Hidden files toggle doesn't reload directory** ✅ FIXED
**Location**: `src/ui/file_picker.rs` lines 301-313

**Problem**:
```rust
onclick: move |_| {
    let show = {
        let mut panes = picker.0.write();
        if let Some(p) = panes.iter_mut().find(|p| p.id == pane_id) {
            p.show_hidden = !p.show_hidden;
            p.columns.clear();  // ❌ Clears but never reloads
            p.show_hidden
        } else {
            false
        }
    };
    let _ = show;  // ❌ This does nothing!
},
```

When user toggles hidden files, columns were cleared but **never reloaded**.

**Applied Fix** (src/ui/file_picker.rs lines 300-331):
```rust
onclick: move |_| {
    // 1. Toggle the flag and clear columns
    {
        let mut panes = picker.0.write();
        if let Some(p) = panes.iter_mut().find(|p| p.id == pane_id) {
            p.show_hidden = !p.show_hidden;
            p.columns.clear();
        }
    }

    // 2. Reload root directory with new show_hidden setting
    spawn(async move {
        let (root, show_hidden) = {
            let panes = picker.0.read();
            panes
                .iter()
                .find(|p| p.id == pane_id)
                .map(|p| (p.root_path.clone(), p.show_hidden))
                .unwrap_or_else(|| (std::path::PathBuf::from("/"), false))
        };
        let entries = read_dir_sorted(&root, show_hidden).await;
        let mut panes = picker.0.write();
        if let Some(p) = panes.iter_mut().find(|p| p.id == pane_id) {
            p.columns = vec![PickerColumn {
                dir_path: root,
                entries,
                selected: None,
            }];
        }
    });
},
```

**Status**: ✅ Applied and compiling

---

## Non-Critical Issues (UX/future work)

### 3. **No visual loading state**
When user clicks a directory and it's reading the filesystem (async), the UI doesn't show a loading indicator. The column content just appears after a delay.

**Impact**: User doesn't know if the click worked or if the app is frozen.

**Fix** (future): Add a small spinner or disabled state while loading.

---

### 4. **Keyboard navigation not implemented**
Design doc 8 mentions keyboard nav, but code doesn't support it yet.

**Deferred to future**.

---

### 5. **Drag-to-workspace not implemented**
Code assumes "Add to workspace" button only. Drag-drop is deferred.

**Deferred to future**.

---

### 6. **No error handling for permission denied, unreadable dirs**
If a directory is unreadable (permission denied), `read_dir_sorted()` just returns an empty Vec silently.

**Impact**: User clicks a folder, nothing appears, no error message.

**Fix** (future): Log errors with tracing, show visual indicator ("Permission denied" message in column).

---

### 7. **Pane minimized tab bar appears even with no panes**
If a pane is open and unminimized, the tab bar doesn't appear.But the check `if has_minimized` at line 195 is correct, so this is not a bug actually.

---

## Summary

**FIXED** ✅:
1. ✅ Lines 171, 198, 203-204, 206-209: Fixed `use_context` calls in minimized tab bar closures + made picker mutable
2. ✅ Lines 300-331: Fixed hidden files toggle to actually reload directory

**Compilation**: ✅ Build succeeds
**Build status**: `/Users/anders/kip/target/dx/kip/debug/macos/Kip.app`

**Deferred to future**:
- Loading spinners
- Keyboard nav
- Drag-drop
- Better error messages

---

## Testing Checklist

After fixes, test:
- [ ] Click "+" → picker opens (no crash)
- [ ] Click folder → navigates to next column with its contents
- [ ] Click minimize button → pane minimizes to bottom tab
- [ ] Click minimized tab → pane restores to original position
- [ ] Right-click minimized tab → pane closes
- [ ] Toggle ".*" button → hidden files show/hide in current column
- [ ] Select file → "Add to workspace" button enables
- [ ] Click "Add to workspace" → location node appears in graph
- [ ] Root level shows files + folders in sorted order
