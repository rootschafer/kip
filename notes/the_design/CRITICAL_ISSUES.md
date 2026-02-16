# Critical Issues & Known Bugs

**Last Updated:** Current Session

---

## Resolved Issues

### ✅ Infinite Loop #1 - Spawns in Component Body
**Severity:** Critical (caused app freeze + 209GB log file)
**Status:** FIXED

**Problem:**
```rust
// In src/app.rs - component body
spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        *refresh_tick.write() += 1;
    }
});
```

Every component re-render created a NEW spawn, multiplying the polling loops. All loops incremented `refresh_tick` simultaneously, causing:
- Resource re-runs
- State updates
- More re-renders
- More spawns
- INFINITE LOOP

**Fix:**
```rust
use_effect(move || {
    spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            *refresh_tick.write() += 1;
        }
    });
});
```

**Applied to:**
- Polling loop (refresh_tick)
- Hostname fetch
- Drive watcher

---

### ✅ Infinite Loop #2 - Resource Updating Signals
**Severity:** Critical
**Status:** FIXED

**Problem:**
```rust
use_resource(move || {
    let graph_val = graph.clone();
    async move {
        let data = load_data().await;
        graph_val.with_mut(|g| g.load(data)); // Triggers re-render!
    }
});
```

Resource captures `graph` signal, updates it inside async block. Update triggers component re-render, which recreates the resource closure, which runs again, which updates graph again... INFINITE LOOP.

**Fix:**
```rust
let loaded_data = use_resource(move || {
    let _tick = refresh_tick;
    async move { load_data().await.ok() }
});

use_effect(move || {
    if let Some(Some(data)) = loaded_data.read().as_ref() {
        graph.with_mut(|g| g.load(data));
    }
});
```

Separate resource (loads data) from effect (updates state). Effect only runs when resource completes.

---

### ✅ Infinite Loop #3 - Signal Capture in Closures
**Severity:** High
**Status:** FIXED

**Problem:**
```rust
use_resource(move || {
    let tick = refresh_tick; // Captures Signal<u32>, not u32 value!
    async move { /* ... */ }
});
```

Capturing the Signal instead of its value means the closure changes on every render, triggering the resource to re-run.

**Fix:**
```rust
use_resource(move || {
    let tick = refresh_tick; // tick is u32 (the VALUE)
    async move {
        let _ = tick; // Use the value
        /* ... */
    }
});
```

---

### ✅ Disk Space Exhaustion - File Logging
**Severity:** Critical (filled entire disk with 209GB log file)
**Status:** FIXED

**Problem:**
```rust
// In src/main.rs
let file_appender = tracing_appender::rolling::never(log_dir, "kip.log");
tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer().with_writer(file_appender))
    .init();
```

Combined with infinite loop logging, this created a 209GB log file.

**Fix:**
```rust
// Console only, WARN level
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::WARN)
    .init();
```

---

### ✅ DbHandle Move Errors
**Severity:** Medium (compile errors)
**Status:** FIXED

**Problem:**
```rust
let db = use_context::<DbHandle>();

use_resource(move || {
    let db_val = db.clone(); // db moved here
    async move { /* ... */ }
});

// Later...
rsx! {
    button {
        onclick: move |_| {
            let db = db.clone(); // ERROR: db already moved!
        }
    }
}
```

**Fix:**
```rust
let db = use_context::<DbHandle>();
let db_for_resource = db.clone(); // Pre-clone for resource

use_resource(move || {
    let db_val = db_for_resource.clone();
    async move { /* ... */ }
});

// db is still available for other uses
```

---

## Current Issues

### ⚠️ Force-Directed Graph Not Working
**Severity:** High
**Status:** NEEDS IMPLEMENTATION

**Symptom:**
- Nodes render in static grid layout
- No physics simulation
- Nodes don't respond to drag
- No automatic layout

**Cause:**
Simulation loop disabled to prevent infinite loops. Physics constants defined but not used.

**Location:**
- `src/ui/graph_store.rs` - Physics constants, `tick()` method
- `src/ui/graph.rs` - Simulation loop (commented out)

**Reference:**
See `external/nexus-node-sync/components/GraphCanvas.tsx` for D3 implementation.

**Next Steps:**
1. Implement proper simulation loop with start/stop control
2. Ensure loop doesn't capture signals incorrectly
3. Test with small node count first (5-10 nodes)
4. Gradually increase to verify performance

---

### ⚠️ Directory Expansion Not Visual
**Severity:** Medium
**Status:** PARTIALLY IMPLEMENTED

**Symptom:**
- `toggle_expand()` exists in Graph struct
- `wake()` method exists to start simulation
- But no visual expansion occurs

**Cause:**
- Simulation not running
- Orbit positioning not implemented
- Node rendering doesn't check `is_expanded` state

**Location:**
- `src/ui/graph_store.rs` - `toggle_expand()`, `wake()`
- `src/ui/graph_nodes.rs` - Node rendering

**Reference:**
See `external/nexus-node-sync/App.tsx` - `handleNodeClick()` expansion logic.

---

### ⚠️ Edge Creation Incomplete
**Severity:** Medium
**Status:** PARTIALLY IMPLEMENTED

**Symptom:**
- Drag state tracks edge creation
- But no visual edge preview
- Edge doesn't complete on drop
- No intent created in database

**Cause:**
- SVG edge preview not implemented
- Mouse up handler incomplete
- Database creation missing

**Location:**
- `src/ui/graph.rs` - Mouse event handlers
- `src/ui/graph_edges.rs` - SVG overlay

**Reference:**
See `external/nexus-node-sync/App.tsx` - `linkMode` and edge creation.

---

## Common Pitfalls (DO NOT REPEAT)

### 1. Spawns in Component Body
```rust
// ❌ NEVER DO THIS
#[component]
fn MyComponent() -> Element {
    spawn(async move { /* ... */ }); // Runs on EVERY render!
    rsx! { /* ... */ }
}

// ✅ CORRECT
#[component]
fn MyComponent() -> Element {
    use_effect(move || {
        spawn(async move { /* ... */ }); // Runs ONCE
    });
    rsx! { /* ... */ }
}
```

### 2. Updating Signals in Resources
```rust
// ❌ NEVER DO THIS
use_resource(move || {
    let signal_val = signal.clone();
    async move {
        let data = load().await;
        signal_val.write().update(data); // Triggers re-render!
    }
});

// ✅ CORRECT
let data = use_resource(move || async move { load().await });
use_effect(move || {
    if let Some(d) = data.read().as_ref() {
        signal.write().update(d);
    }
});
```

### 3. Excessive Logging
```rust
// ❌ NEVER DO THIS
info!("Loading..."); // In a loop = GBs of logs
debug!("Tick: {}", count); // Every frame = millions of lines

// ✅ CORRECT
trace!("Tick: {}", count); // Only shows with trace level
// Or better, don't log in tight loops at all
```

### 4. Signal vs Value Capture
```rust
// ❌ WRONG
use_resource(move || {
    let x = my_signal; // x is Signal<T>
    async move { /* ... */ }
});

// ✅ CORRECT
use_resource(move || {
    let x = my_signal(); // x is T (the value)
    async move { /* ... */ }
});
```

---

## Debugging Tips

### Detecting Infinite Loops

1. **Watch CPU usage** - Spikes to 100% immediately
2. **Watch log file size** - Grows rapidly (GBs per minute)
3. **Add counter logging:**
   ```rust
   static COUNTER: AtomicUsize = AtomicUsize::new(0);
   let count = COUNTER.fetch_add(1, Ordering::Relaxed);
   eprintln!("Render count: {}", count); // Should stabilize
   ```

### Finding Signal Loops

1. **Check what triggers re-render:**
   - Signal read in component body?
   - Signal updated in resource/effect?
   - Does update trigger same signal again?

2. **Use `use_effect` dependencies:**
   ```rust
   use_effect(move || {
       // This runs when `value` changes
       let v = my_signal();
       println!("Changed to: {}", v);
   });
   ```

### Performance Issues

1. **Count re-renders:**
   ```rust
   static COUNT: AtomicUsize = AtomicUsize::new(0);
   let c = COUNT.fetch_add(1, Ordering::Relaxed);
   if c % 100 == 0 { eprintln!("{} renders", c); }
   ```

2. **Check for unnecessary clones:**
   - Cloning large data structures in render
   - Cloning in loops

3. **Profile with `tokio-console`:**
   ```bash
   cargo install tokio-console
   # Add tokio-console feature to Cargo.toml
   # Run app, then: tokio-console
   ```

---

## Emergency Recovery

### If App Freezes

1. **Kill the process:** `Ctrl+C` or `killall kip`
2. **Check log file:** `ls -lh ~/Library/Application\ Support/Kip/kip.log`
3. **Delete log if huge:** `rm ~/Library/Application\ Support/Kip/kip.log`
4. **Check disk space:** `df -h /`
5. **Clean build:** `rm -rf target && dx build`

### If Disk Is Full

1. **Find large files:**
   ```bash
   du -ah / | sort -rh | head -20
   ```
2. **Delete target directory:** `rm -rf /Users/anders/kip/target`
3. **Delete log files:** `rm -rf /Users/anders/kip/*.log`
4. **Empty trash**

---

## Contact / Escalation

If you encounter a new critical issue:

1. **Document it immediately** in this file
2. **Add reproduction steps**
3. **Add fix if found**
4. **Update START_HERE.md** if it's a common pitfall

**DO NOT** leave critical issues undocumented - the next AI will hit the same problem.
