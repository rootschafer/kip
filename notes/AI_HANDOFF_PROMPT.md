# AI Handoff Prompt: Force-Directed Graph Implementation

## Your Task

Implement a **force-directed graph layout** for the Kip file synchronization app. You will port concepts from a TypeScript/D3 reference implementation to Rust/Dioxus.

---

## 🚨 CRITICAL: Read These First

**DO NOT START CODING UNTIL YOU HAVE:**

1. ✅ Read `notes/the_design/START_HERE.md` - Project overview and critical pitfalls
2. ✅ Read `notes/the_design/CRITICAL_ISSUES.md` - Known bugs and infinite loop fixes
3. ✅ Read `notes/the_design/IMPLEMENTATION_SUMMARY.md` - Current implementation state
4. ✅ Studied `external/nexus-node-sync/components/GraphCanvas.tsx` - D3 force-directed reference
5. ✅ Studied `external/nexus-node-sync/App.tsx` - Interaction logic reference
6. ✅ Understood the infinite loop patterns that broke the app before

**IF YOU SKIP THESE STEPS, YOU WILL BREAK THE APP.**

---

## Project Context

**What is Kip?**
Kip is a file synchronization orchestrator. Users connect devices (local machines, remote servers via SSH, mounted drives) and create sync relationships between folders. The primary UI is a force-directed graph where:
- **Nodes** = devices, folders, files
- **Edges** = sync relationships
- **Interactions** = drag to move, click to expand, drag-between-nodes to create sync

**Tech Stack:**
- **Frontend:** Dioxus (Rust React-like framework)
- **Backend:** SurrealDB (embedded database)
- **Graphics:** SVG overlay for edges, HTML divs for nodes
- **State:** Dioxus signals and stores

**Current State:**
- ✅ App builds and runs without freezing
- ✅ Basic graph structure exists (nodes render, edges render)
- ✅ Database integration works
- ❌ Force-directed layout NOT working (static grid layout currently)
- ❌ Directory expansion NOT visual
- ❌ Edge creation incomplete

---

## Reference Implementation

**Location:** `external/nexus-node-sync/`

This is a complete TypeScript/React implementation using D3.js. Study these files:

### 1. `types.ts` - Data Structures
```typescript
interface FileNode {
  id: string;
  name: string;
  type: 'root' | 'device' | 'folder' | 'file' | 'group';
  x?: number;
  y?: number;
  fx?: number | null;  // Fixed position (during drag)
  fy?: number | null;
  // ... more fields
}
```

**Your equivalent:** `src/ui/graph_types.rs` - `GraphNode` struct

### 2. `GraphCanvas.tsx` - Force-Directed Graph
Key D3 configuration:
```typescript
d3.forceSimulation(nodes)
  .force("link", d3.forceLink(links)
    .id(d => d.id)
    .distance(d => d.type === 'sync' ? 150 : 80))
  .force("charge", d3.forceManyBody().strength(-300))
  .force("x", d3.forceX().strength(0.04))
  .force("y", d3.forceY().strength(0.04))
  .force("collide", d3.forceCollide()
    .radius(d => d.type === 'device' ? 45 : 30)
    .iterations(2))
```

**Your job:** Port this logic to Rust in `src/ui/graph_store.rs`

### 3. `App.tsx` - Interaction Logic
Key patterns:
- Click node → expand if directory
- Shift+click → multi-select
- Link mode → create sync edge
- Drag node → move with physics
- Shift+drag background → lasso select

**Your equivalent:** `src/ui/graph.rs` - mouse event handlers

---

## Implementation Plan

### Phase 1: Physics Simulation (4-6 hours)

**Goal:** Nodes arrange themselves automatically based on forces.

**Steps:**
1. **Choose approach:**
   - Option A: Use Rust crate (`force-graph`, `petgraph`)
   - Option B: Port D3 logic directly (recommended for exact match)

2. **Implement forces in `src/ui/graph_store.rs`:**
   - `apply_repulsion()` - nodes push apart
   - `apply_link_forces()` - connected nodes pull together
   - `apply_center_gravity()` - gentle pull to center
   - `resolve_collisions()` - prevent overlap

3. **Add simulation loop:**
   - Use `use_effect` (NOT direct spawn - see CRITICAL_ISSUES.md)
   - Tick at ~60fps
   - Stop when settled (alpha < 0.001)
   - Restart on changes

4. **Test:**
   - Start with 5 nodes
   - Verify nodes spread out
   - Verify connected nodes stay close
   - Check CPU usage (should be low when settled)

**Reference:** `notes/the_design/Phase1/Phase1.3_Force_Directed_Layout_Implementation.md`

### Phase 2: Drag-to-Move (2-3 hours)

**Goal:** User can drag nodes, physics responds.

**Steps:**
1. **Add fixed position fields to `GraphNode`:**
   ```rust
   pub fx: Option<f64>,  // Fixed x (during drag)
   pub fy: Option<f64>,  // Fixed y
   pub vx: f64,  // Velocity x
   pub vy: f64,  // Velocity y
   ```

2. **Update mouse handlers in `src/ui/graph.rs`:**
   - On drag start: set `fx`, `fy` to current position
   - On drag move: update `fx`, `fy` and position
   - On drag end: clear `fx`, `fy`, restart simulation

3. **Update integration in `tick()`:**
   - Skip nodes with `fx`/`fy` set
   - They stay fixed during simulation

4. **Test:**
   - Drag single node
   - Node should follow cursor
   - Other nodes should react
   - Node should settle when released

### Phase 3: Directory Expansion (3-4 hours)

**Goal:** Click directory → children appear in orbit.

**Steps:**
1. **Implement orbit positioning:**
   ```rust
   fn calculate_orbit_positions(
       parent_x: f64,
       parent_y: f64,
       children: &[&GraphNode],
       radius: f64,
   ) -> Vec<(String, f64, f64)>
   ```

2. **Update `toggle_expand()` in `src/ui/graph_store.rs`:**
   - Set `is_expanded = true`
   - Calculate orbit positions for children
   - Start simulation

3. **Update node rendering:**
   - Check `is_expanded` state
   - Show children in orbit
   - Animate expansion

4. **Test:**
   - Click directory
   - Children should appear in circle around parent
   - Should animate smoothly

### Phase 4: Polish (2-3 hours)

**Goal:** Visual polish and performance.

**Steps:**
1. **Node sizing based on descendants:**
   ```rust
   fn calculate_node_size(descendants: usize) -> f64 {
       let log_count = (1.0 + descendants as f64).ln();
       (80.0 + log_count * 15.0).clamp(60.0, 150.0)
   }
   ```

2. **Edge lengths by type:**
   - Sync edges: 150px
   - Hierarchy edges: 80px
   - Group edges: 60px

3. **Collision radii by node type:**
   - Device: 45px
   - Directory/Group: 30px
   - File: 15px

4. **Test with 100+ nodes:**
   - Should maintain 60fps
   - Should settle reasonably fast
   - No crashes

---

## ⚠️ CRITICAL: Infinite Loop Prevention

**YOU MUST FOLLOW THESE RULES OR THE APP WILL FREEZE:**

### Rule 1: Always Wrap Spawns in use_effect

```rust
// ❌ WRONG - Creates new spawn on EVERY render
#[component]
fn MyComponent() -> Element {
    spawn(async move {
        loop {
            do_something().await;
        }
    });
    rsx! { /* ... */ }
}

// ✅ CORRECT - Spawns only once
#[component]
fn MyComponent() -> Element {
    use_effect(move || {
        spawn(async move {
            loop {
                do_something().await;
            }
        });
    });
    rsx! { /* ... */ }
}
```

### Rule 2: Don't Update Signals from Inside use_resource

```rust
// ❌ WRONG - Resource updates signal, triggers re-render, recreates resource
use_resource(move || {
    let graph_val = graph.clone();
    async move {
        let data = load().await;
        graph_val.with_mut(|g| g.load(data)); // TRIGGERS INFINITE LOOP
    }
});

// ✅ CORRECT - Separate resource and effect
let loaded_data = use_resource(move || async move {
    load().await.ok()
});

use_effect(move || {
    if let Some(Some(data)) = loaded_data.read().as_ref() {
        graph.with_mut(|g| g.load(data));
    }
});
```

### Rule 3: Capture Values, Not Signals

```rust
// ❌ WRONG - Captures Signal<u32>, closure changes every render
use_resource(move || {
    let tick = refresh_tick; // tick is Signal<u32>
    async move { /* ... */ }
});

// ✅ CORRECT - Captures u32 value
use_resource(move || {
    let tick = refresh_tick; // tick is u32 (the VALUE)
    async move {
        let _ = tick; // Use the value
        /* ... */
    }
});
```

### Rule 4: No File Logging

```rust
// ❌ WRONG - Created 209GB log file
tracing_subscriber::registry()
    .with(tracing_subscriber::fmt().layer().with_writer(file_appender))
    .init();

// ✅ CORRECT - Console only, WARN level
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::WARN)
    .init();
```

**Already fixed in:** `src/main.rs`

---

## Testing Checklist

After each phase, verify:

- [ ] App doesn't freeze (run for 2+ minutes)
- [ ] CPU usage is reasonable (< 20% when idle)
- [ ] No console errors
- [ ] Log file not growing (`ls -lh ~/Library/Application\ Support/Kip/kip.log`)
- [ ] Interactions work (click, drag, etc.)
- [ ] Visual looks correct

**If CPU spikes or log file grows:**
1. Kill app immediately
2. Check what you changed
3. Review infinite loop patterns in CRITICAL_ISSUES.md
4. Fix before continuing

---

## File Locations

### Core Files to Modify
- `src/ui/graph_store.rs` - Graph struct, physics simulation
- `src/ui/graph.rs` - Component, mouse handlers, simulation loop
- `src/ui/graph_types.rs` - Node/Edge types (add fx, fy, vx, vy fields)
- `src/ui/graph_nodes.rs` - Node rendering (optional polish)

### Reference Files
- `external/nexus-node-sync/components/GraphCanvas.tsx` - D3 implementation
- `external/nexus-node-sync/App.tsx` - Interaction logic
- `external/nexus-node-sync/types.ts` - Type definitions

### Documentation
- `notes/the_design/START_HERE.md` - Project overview
- `notes/the_design/CRITICAL_ISSUES.md` - Known bugs
- `notes/the_design/IMPLEMENTATION_SUMMARY.md` - Current state
- `notes/the_design/Phase1/Phase1.3_Force_Directed_Layout_Implementation.md` - Detailed plan

---

## Build Commands

```bash
# Build the app
dx build

# Run in development mode
dx serve --platform desktop

# Check without building
dx check
```

**DO NOT use `cargo build` or `cargo check`** - Dioxus projects must use `dx` commands.

---

## When You Get Stuck

1. **Check documentation first:**
   - START_HERE.md for general patterns
   - CRITICAL_ISSUES.md for bugs
   - Phase1.3_Force_Directed_Layout_Implementation.md for physics details

2. **Compare with TypeScript reference:**
   - How does D3 handle this?
   - Can you port the logic directly?

3. **Document new issues:**
   - Add to CRITICAL_ISSUES.md
   - Include reproduction steps
   - Include fix if found

4. **Update documentation:**
   - When you fix something, update the docs
   - When you add a feature, update IMPLEMENTATION_SUMMARY.md
   - Help the next person (or future you)

---

## Success Criteria

Your implementation is complete when:

1. **Physics Works:**
   - Nodes spread out automatically
   - Connected nodes stay close
   - Graph settles (doesn't jitter forever)

2. **Drag Works:**
   - Can drag nodes smoothly
   - Other nodes react to drag
   - Released nodes settle into place

3. **Expansion Works:**
   - Click directory → children appear
   - Children arranged in orbit
   - Smooth animation

4. **No Infinite Loops:**
   - App runs for 5+ minutes without freezing
   - CPU stays reasonable
   - Log file stays small (< 10MB)

5. **Performance:**
   - 60fps with 50 nodes
   - 30+ fps with 200 nodes
   - No memory leaks

---

## Good Luck!

You have everything you need:
- ✅ Working app structure
- ✅ Reference implementation
- ✅ Detailed documentation
- ✅ Known pitfalls documented

**Just follow the rules, test frequently, and document as you go.**

If you hit an issue, it's already documented in CRITICAL_ISSUES.md. If it's a NEW issue, add it there so the next person doesn't hit the same problem.

**Happy coding! 🚀**
