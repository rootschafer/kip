# Kip Development: Getting Started

## ⚠️ CRITICAL: Read This First

**DO NOT start coding until you have:**
1. Read this entire document
2. Read `CRITICAL_ISSUES.md` - ALL infinite loop patterns and fixes
3. Read `IMPLEMENTATION_SUMMARY.md` - Current implementation state
4. Reviewed the TypeScript reference in `external/nexus-node-sync/`

## Project Overview

Kip is a file synchronization orchestrator built with **Dioxus (Rust)** + **SurrealDB**. The primary interface is a **force-directed graph** where users visualize and manage file sync relationships between devices, folders, and files.

## Reference Implementation

**Location:** `external/nexus-node-sync/`

A complete TypeScript/React implementation using D3.js force-directed graphs. Your job is to **port concepts and patterns** to Dioxus/Rust.

**Key files to study:**
- `external/nexus-node-sync/types.ts` - Node/Link data structures
- `external/nexus-node-sync/App.tsx` - Main app logic, interaction handlers
- `external/nexus-node-sync/components/GraphCanvas.tsx` - **D3 force-directed graph**
- `external/nexus-node-sync/services/mockFileSystem.ts` - Mock data generation

## Current State (February 2026)

### ✅ COMPLETED - Force-Directed Graph
- **Physics simulation** - Repulsion, link attraction, center gravity, collision
- **Infinite canvas** - Alt+drag to pan, nodes spread infinitely
- **Cluster separation** - Machines/drives form separate visual clusters
- **All nodes connected** - Hierarchy edges for parent-child relationships
- **Drag-to-move** - Fix/release positions with D3-style fx/fy
- **Edge preview** - Rubber band line follows cursor during edge creation
- **Cluster backgrounds** - Faint colored circles around each machine/drive
- **Filesystem scanning** - Alt+click machine/drive to scan and populate nodes

### ✅ COMPLETED - Infrastructure
- Database layer (SurrealDB)
- App structure with proper signal management
- File picker component
- Notification system
- Review queue

### ❌ INCOMPLETE / NEEDS WORK

1. **Zoom** - Scroll wheel zoom not working (Dioxus API issue)
2. **Directory expansion** - Click to expand folders (exists but needs filesystem integration)
3. **Edge creation** - Drag between nodes to create sync (UI exists, DB creation missing)
4. **Lasso selection** - Shift+drag to select multiple (UI exists, multi-drag missing)
5. **Node visuals** - Circle vs pill shapes, gradients, status indicators
6. **Node grouping** - Group selected nodes into container

## 🐛 CRITICAL: Infinite Loop Patterns (DO NOT REPEAT)

### Pattern 1: Spawns in Component Body
```rust
// ❌ WRONG - Creates new spawn on EVERY render
#[component]
fn MyComponent() -> Element {
    spawn(async move { loop { /* ... */ } });
    rsx! { /* ... */ }
}

// ✅ CORRECT - Wrap in use_effect
#[component]
fn MyComponent() -> Element {
    use_effect(move || {
        spawn(async move { loop { /* ... */ } });
    });
    rsx! { /* ... */ }
}
```

### Pattern 2: Resource Updating Signals
```rust
// ❌ WRONG - Resource updates signal, triggers re-render, recreates resource
use_resource(move || {
    let graph_val = graph.clone();
    async move {
        let data = load().await;
        graph_val.with_mut(|g| g.load(data)); // INFINITE LOOP
    }
});

// ✅ CORRECT - Separate resource and effect
let loaded_data = use_resource(move || async move { load().await.ok() });
use_effect(move || {
    if let Some(Some(data)) = loaded_data.read().as_ref() {
        graph.with_mut(|g| g.load(data));
    }
});
```

### Pattern 3: Signal vs Value Capture
```rust
// ❌ WRONG - Captures Signal<T>, closure changes every render
use_resource(move || {
    let tick = refresh_tick; // tick is Signal<u32>
    async move { /* ... */ }
});

// ✅ CORRECT - Capture the VALUE
use_resource(move || {
    let tick = refresh_tick; // tick is u32 (the VALUE)
    async move {
        let _ = tick; // Use the value
        /* ... */
    }
});
```

### Pattern 4: File Logging
```rust
// ❌ WRONG - Created 209GB log file
tracing_subscriber::fmt()
    .with_writer(file_appender)
    .init();

// ✅ CORRECT - Console only, WARN level
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::WARN)
    .init();
```

## Architecture

### Key Components
- `src/main.rs` - Entry point, logging (console only, WARN level)
- `src/app.rs` - Root component, global state
- `src/ui/graph.rs` - Main graph component
- `src/ui/graph_store.rs` - Graph state, physics simulation
- `src/ui/graph_nodes.rs` - Node rendering
- `src/ui/graph_edges.rs` - SVG edge overlay
- `src/ui/graph_types.rs` - Type definitions

### Database Schema
- `machine` - Computers (local/remote)
- `drive` - Mounted drives
- `location` - File paths
- `intent` - Sync relationships
- `review_item` - Conflicts

## Build Commands

```bash
dx build          # Build the app
dx serve          # Run in dev mode
dx check          # Check without building
```

**DO NOT use `cargo build`** - Dioxus projects must use `dx` commands.

## Development Guidelines

### Dioxus Patterns
- `use_signal<T>` - Local component state
- `use_store` - Global app state
- `use_context` - Shared resources (DbHandle)
- `use_resource` - Async data loading
- `use_effect` - Side effects (spawns, subscriptions)

### Force-Directed Graph Constants
Current tuned values in `src/ui/graph_store.rs`:
```rust
REPULSION: 2000.0      // Strong cluster separation
SPRING_K: 0.03         // Weak link tension
CENTER_GRAVITY: 0.003  // Very weak center pull
ALPHA_DECAY: 0.97      // Slow decay for settling
```

## Files to Update

When making changes, update:
- `IMPLEMENTATION_SUMMARY.md` - Current state
- `CRITICAL_ISSUES.md` - New bugs/gotchas
- `Phase*/` directories - Feature-specific docs

## Contact

Document ALL issues in `CRITICAL_ISSUES.md` with:
- What you were trying to do
- What actually happened
- Code snippets
- Fix if found

**DO NOT leave issues undocumented** - the next developer will hit the same problem.
