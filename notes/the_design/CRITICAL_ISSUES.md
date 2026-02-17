# Critical Issues & Known Bugs

**Last Updated:** February 17, 2026

---

## RESOLVED - Critical Issues

### ✅ Infinite Loop #1 - Spawns in Component Body
**Severity:** Critical (caused app freeze + 209GB log file)  
**Status:** FIXED

**Problem:**
```rust
// In component body - creates new spawn on EVERY render
spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        *refresh_tick.write() += 1;
    }
});
```

**Fix:**
```rust
use_effect(move || {
    spawn(async move {
        loop { /* ... */ }
    });
});
```

**Applied to:**
- Polling loop (refresh_tick)
- Hostname fetch
- Drive watcher
- Simulation loop

---

### ✅ Infinite Loop #2 - Resource Updating Signals
**Severity:** Critical  
**Status:** FIXED

**Problem:**
```rust
use_resource(move || {
    let graph_val = graph.clone();
    async move {
        graph_val.with_mut(|g| g.load(data)); // Triggers re-render!
    }
});
```

**Fix:**
```rust
let loaded_data = use_resource(move || async move { load().await.ok() });
use_effect(move || {
    if let Some(Some(data)) = loaded_data.read().as_ref() {
        graph.with_mut(|g| g.load(data));
    }
});
```

---

### ✅ Infinite Loop #3 - Signal vs Value Capture
**Severity:** High  
**Status:** FIXED

**Problem:**
```rust
use_resource(move || {
    let tick = refresh_tick; // Captures Signal<u32>
    async move { /* ... */ }
});
```

**Fix:**
```rust
use_resource(move || {
    let tick = refresh_tick; // tick is u32 VALUE
    async move {
        let _ = tick; // Use the value
        /* ... */
    }
});
```

---

### ✅ Disk Space Exhaustion - File Logging
**Severity:** Critical (209GB log file)  
**Status:** FIXED

**Fix in `src/main.rs`:**
```rust
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::WARN)
    .init();
```

---

### ✅ Simulation Loop Not Restarting
**Severity:** High (nodes appeared but didn't move)  
**Status:** FIXED

**Problem:** Simulation loop would `break` when alpha dropped, then not restart when new nodes added.

**Fix:** Loop never breaks, just resets tick_count and waits for sim_running:
```rust
if !should_continue {
    tracing::info!("Simulation loop: tick {} stopped, will restart if needed", tick_count);
    tick_count = 0;  // Reset, don't break!
}
```

---

### ✅ Viewport Transform Missing
**Severity:** Medium (edge preview misaligned)  
**Status:** FIXED

**Problem:** Rubber band line didn't account for viewport transform.

**Fix in `graph_edges.rs`:**
```rust
let graph_mouse_x = (mouse_x - viewport_x) / viewport_scale;
let graph_mouse_y = (mouse_y - viewport_y) / viewport_scale;
```

---

### ✅ Cluster Attraction
**Severity:** Medium (nodes from different machines clumped)  
**Status:** FIXED

**Problem:** Repulsion too weak, center gravity too strong.

**Fix - Tuned constants in `graph_store.rs`:**
```rust
REPULSION: 2000.0      // Was 300-800, increased for separation
SPRING_K: 0.03         // Was 0.05-0.1, reduced tension
CENTER_GRAVITY: 0.003  // Was 0.02-0.04, very weak now
```

---

## CURRENT ISSUES

### ⚠️ Zoom Not Working
**Severity:** Medium  
**Status:** BLOCKED - Dioxus API incompatibility

**Symptom:**
- Scroll wheel does nothing
- Various API attempts fail (`delta_y()`, field access, enum match)

**Location:** `src/ui/graph.rs` - wheel handler removed

**Attempts:**
```rust
// All failed:
e.delta_y()           // Method doesn't exist
e.data().delta_y()    // Method doesn't exist
e.data().y            // Field doesn't exist
e.data().delta        // Returns WheelDelta enum
match e.data().delta { /* variants not accessible */ }
```

**Workaround:** Alt+drag pan works fine

**Fix Needed:**
- Find correct Dioxus wheel event API for version 0.7.3
- Or add zoom buttons (+/-)
- Or implement keyboard zoom (Ctrl+scroll, +/-)

---

### ⚠️ Directory Expansion Incomplete
**Severity:** Medium  
**Status:** PARTIAL

**What Works:**
- `toggle_expand()` sets expanded state
- Finds children by parent_id
- Works for machine/drive nodes (filesystem scan)

**What Doesn't:**
- Directory nodes from DB don't trigger filesystem scan
- Only machines/drives scan filesystem
- Clicking a directory node does nothing if no children in DB

**Location:** `src/ui/graph.rs` - expansion handler

**Fix Needed:**
Extend scan logic to Directory nodes:
```rust
if is_directory && !has_children_in_db {
    scan_filesystem_directory(path);
}
```

---

### ⚠️ Edge Creation Incomplete
**Severity:** Medium  
**Status:** PARTIAL

**What Works:**
- Ctrl/Alt+click starts edge creation
- Rubber band line follows cursor
- DragState::CreatingEdge tracks state

**What Doesn't:**
- Dropping on target node does nothing
- No intent created in database
- No edge added to graph

**Location:** `src/ui/graph.rs` - onmouseup handler

**Fix Needed:**
```rust
// In node mousedown handler during CreatingEdge state:
if let DragState::CreatingEdge { source_id } = &drag_state {
    create_edge_in_db(&db, source_id, &target_id).await;
    graph.with_mut(|g| g.add_edge(new_edge));
}
```

---

### ⚠️ Lasso Multi-Drag Missing
**Severity:** Low  
**Status:** NOT IMPLEMENTED

**What Works:**
- Shift+drag creates selection rectangle
- Nodes selected in rect

**What Doesn't:**
- Selected nodes don't move together
- Can only drag one node at a time

**Location:** `src/ui/graph.rs` - DragState::Dragging handler

**Fix Needed:**
Track all selected nodes and apply same offset to each.

---

## ARCHITECTURAL ISSUES

### ⚠️ Filesystem Scan Coupling
**Severity:** Low  
**Status:** DESIGN DECISION

**Current:** `scan_directory()` creates nodes directly, called from UI

**Better Pattern:** 
- Scan service returns node data
- Graph state updated via action/reducer pattern
- Separation of concerns

**Why Not Fixed:** Works correctly, refactor can wait

---

### ⚠️ Graph State Monolith
**Severity:** Low  
**Status:** ACCEPTABLE

**Current:** All graph state in single `Graph` struct

**Better Pattern:**
- Separate signals for nodes, edges, viewport, drag_state
- More granular reactivity

**Why Not Fixed:** Single signal is simpler, performance is fine

---

### ⚠️ CSS Hardcoded Values
**Severity:** Low  
**Status:** TECHNICAL DEBT

**Example:**
```css
.header { padding: 16px 24px; }  /* Hardcoded */
.workspace-svg { z-index: 1; }   /* Magic number */
```

**Better:** CSS custom properties for theme values

**Why Not Fixed:** Visual polish, not blocking

---

## DEBUGGING PATTERNS

### Detecting Infinite Loops

1. **CPU spike** - Immediately goes to 100%
2. **Log file growth** - `ls -lh ~/Library/Application\ Support/Kip/kip.log`
3. **Counter logging:**
   ```rust
   static COUNTER: AtomicUsize = AtomicUsize::new(0);
   let count = COUNTER.fetch_add(1, Ordering::Relaxed);
   if count % 100 == 0 { tracing::info!("Render {}", count); }
   ```

### Finding Signal Loops

1. Check what triggers re-render:
   - Signal read in component body?
   - Signal updated in resource/effect?
   - Does update trigger same signal again?

2. Use `use_effect` to track changes:
   ```rust
   use_effect(move || {
       let v = my_signal();
       tracing::info!("Changed to: {}", v);
   });
   ```

### Viewport Issues

If nodes don't align with mouse:
```rust
// Always transform:
let graph_x = (mouse_x - viewport_x) / viewport_scale;
let graph_y = (mouse_y - viewport_y) / viewport_scale;
```

### Force Tuning

If clusters clump together:
- Increase REPULSION (currently 2000)
- Decrease CENTER_GRAVITY (currently 0.003)
- Increase edge lengths (currently 180-250px)

---

## EMERGENCY RECOVERY

### If App Freezes

1. **Kill:** `Ctrl+C` or `killall kip`
2. **Check log:** `ls -lh ~/Library/Application\ Support/Kip/kip.log`
3. **Delete if huge:** `rm ~/Library/Application\ Support/Kip/kip.log`
4. **Check disk:** `df -h /`
5. **Clean build:** `rm -rf target && dx build`

### If Disk Is Full

1. **Find large files:**
   ```bash
   du -ah / | sort -rh | head -20
   ```
2. **Delete target:** `rm -rf /Users/anders/kip/target`
3. **Delete logs:** `rm -rf /Users/anders/kip/*.log`
4. **Empty trash**

---

## CONTACT / ESCALATION

If you encounter a new critical issue:

1. **Document immediately** in this file
2. **Add reproduction steps**
3. **Add fix if found**
4. **Update START_HERE.md** if it's a common pattern

**DO NOT** leave issues undocumented - the next developer WILL hit the same problem.

### Template for New Issues

```markdown
### ⚠️ [Issue Name]
**Severity:** [Critical/High/Medium/Low]  
**Status:** [NEW/INVESTIGATING/FIXED]

**Symptom:**
[What you see]

**Cause:**
[Root cause if known]

**Location:**
[File and line numbers]

**Fix:**
[Solution if found]

**Reference:**
[Related files or patterns]
```
