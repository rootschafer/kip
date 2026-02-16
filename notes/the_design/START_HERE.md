# Kip Development: Getting Started

## ⚠️ CRITICAL: Read This First

**DO NOT start coding until you have:**
1. Read this entire document
2. Reviewed the TypeScript reference implementation in `external/nexus-node-sync/`
3. Understood the infinite loop pitfalls documented below
4. Reviewed the current state of implementation

## Project Overview

Kip is a file synchronization orchestrator built with **Dioxus (Rust)** + **SurrealDB**. The primary interface is a **force-directed graph** where users visualize and manage file sync relationships between devices, folders, and files.

## Reference Implementation

**Location:** `external/nexus-node-sync/`

A complete TypeScript/React implementation using D3.js force-directed graphs exists. Your job is to **port the concepts and patterns** (not copy code directly) to Dioxus/Rust.

**Key files to study:**
- `external/nexus-node-sync/types.ts` - Node/Link data structures
- `external/nexus-node-sync/App.tsx` - Main app logic, state management, interaction handlers
- `external/nexus-node-sync/components/GraphCanvas.tsx` - **D3 force-directed graph implementation**
- `external/nexus-node-sync/services/mockFileSystem.ts` - Mock data generation

## Current State

### ✅ Completed
- Basic Dioxus app structure with SurrealDB integration
- Database schema for machines, drives, locations, intents (sync rules), review_items
- File picker component for selecting locations
- Notification system with toast notifications
- Review queue for conflict resolution
- Graph component structure (nodes, edges, toolbar)

### ❌ NOT Working / Needs Implementation
- **Force-directed graph layout** - Currently using static grid layout
- **Node expansion** - Directory circles should expand to show children
- **Orbit view** - Children should fan out around parent nodes
- **Node grouping** - Select multiple nodes → group into collapsible container
- **Edge creation** - Drag from node to node to create sync relationship
- **Lasso selection** - Shift+drag to select multiple nodes
- **Proper node rendering** - Nodes should be circles (directories/groups) or pills (files)

### 🐛 Critical Issues Fixed (DO NOT REPEAT)

**Infinite Loop #1 - Spawns in Component Body**
```rust
// ❌ WRONG - Creates new spawn on EVERY render
spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        *refresh_tick.write() += 1;
    }
});

// ✅ CORRECT - Wrap in use_effect so it only runs once
use_effect(move || {
    spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            *refresh_tick.write() += 1;
        }
    });
});
```

**Infinite Loop #2 - Resource Updating Signals**
```rust
// ❌ WRONG - Resource captures graph signal, creates loop
use_resource(move || {
    let graph_val = graph.clone();
    async move {
        let data = load_data().await;
        graph_val.with_mut(|g| g.load(data)); // Triggers re-render!
    }
});

// ✅ CORRECT - Separate resource and effect
let loaded_data = use_resource(move || {
    async move { load_data().await.ok() }
});

use_effect(move || {
    if let Some(data) = loaded_data.read().as_ref() {
        graph.with_mut(|g| g.load(data));
    }
});
```

**Infinite Loop #3 - Signal Capture in Closures**
```rust
// ❌ WRONG - Captures signal, not value
use_resource(move || {
    let _tick = refresh_tick; // Captures the Signal<u32>
    async move { ... }
});

// ✅ CORRECT - Capture the value
use_resource(move || {
    let tick = refresh_tick; // Captures the u32 VALUE
    async move {
        let _ = tick; // Use it to create dependency
        ...
    }
});
```

**Disk Space Issue - Logging**
```rust
// ❌ WRONG - File logging enabled (created 209GB log file!)
tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer().with_writer(file_appender))
    .init();

// ✅ CORRECT - Console only, WARN level
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::WARN)
    .init();
```

## Architecture

### Data Flow
```
User Action → EventHandler → Signal Update → use_effect → Database/Resource → Signal Update → UI Re-render
```

### Key Components
- `src/main.rs` - App entry point, logging setup
- `src/app.rs` - Root component, global state (refresh_tick, picker, notifications)
- `src/ui/graph.rs` - Main graph component (MappingGraph)
- `src/ui/graph_store.rs` - Graph state management (Graph struct)
- `src/ui/graph_nodes.rs` - Node rendering components
- `src/ui/graph_edges.rs` - SVG edge overlay
- `src/ui/graph_types.rs` - Type definitions (GraphNode, GraphEdge, etc.)
- `src/ui/file_picker.rs` - File picker for selecting locations
- `src/ui/notification.rs` - Toast notification system

### Database Schema (SurrealDB)
- `machine` - Computers (local or remote via SSH)
- `drive` - Mounted drives
- `location` - File paths on machines/drives
- `intent` - Sync relationships between locations
- `review_item` - Conflicts requiring user resolution

## Development Guidelines

### Dioxus Patterns

**State Management:**
- Use `use_signal<T>` for local component state
- Use `use_store(|| Store::new())` for global app state
- Use `use_context::<T>()` to access shared resources (like DbHandle)
- Use `use_resource` for async data loading
- Use `use_effect` for side effects (spawns, subscriptions)

**Event Handlers:**
```rust
#[component]
pub fn MyComponent(
    on_click: EventHandler<MouseEvent>,
    on_data: EventHandler<String>,
) -> Element {
    rsx! {
        button {
            onclick: move |e| on_click.call(e),
            "Click me"
        }
    }
}
```

**Async Operations:**
```rust
spawn(async move {
    let result = some_async_operation().await;
    // Update state
    state.write().value = result;
});
```

### Force-Directed Graph Requirements

**Node Types:**
- **Circles** - Directories, groups, devices (expandable)
- **Pills** - Files (leaf nodes)
- **Size** - Based on descendant count (logarithmic scale)

**Interactions:**
- **Click circle** - Expand to show children (orbit view first, then enter)
- **Drag node** - Move node (physics simulation)
- **Shift+click** - Multi-select
- **Shift+drag background** - Lasso selection
- **Drag from node to node** - Create sync edge
- **Right-click** - Context menu

**Physics Forces:**
- **Repulsion** - Nodes push apart (forceManyBody)
- **Link attraction** - Connected nodes pull together (forceLink)
- **Center gravity** - Gentle pull toward center (forceX/forceY)
- **Collision** - Prevent overlap (forceCollide)

**Visual States:**
- Selected - Blue glow with dashed border
- Link mode - Crosshair cursor, edge preview
- Expanding - Children animate outward
- Syncing - Edge pulses with color

## Next Steps

1. **Study the TypeScript implementation** in `external/nexus-node-sync/`
   - Understand how D3 force simulation is configured
   - Note how node/link data is structured
   - Review interaction handlers (click, drag, lasso)

2. **Review current Dioxus implementation**
   - Check `src/ui/graph_store.rs` for Graph struct
   - Review `src/ui/graph_nodes.rs` for node rendering
   - Check `src/ui/graph_edges.rs` for edge rendering

3. **Implement force-directed layout**
   - Port D3 physics concepts to Rust (or use existing Rust force library)
   - Update node positioning logic
   - Add animation for node expansion

4. **Test thoroughly**
   - Ensure no infinite loops (watch for re-renders)
   - Verify performance with 100+ nodes
   - Test all interactions

## Files to Update

When making changes, update these design docs:
- `KIP_DESIGN_7_MAPPING_GRAPH.md` - Graph UI specification
- `Phase1/Phase1.3_Force_Directed_Layout_Implementation.md` - Force layout details
- `IMPLEMENTATION_SUMMARY.md` - Current implementation status
- `CRITICAL_ISSUES.md` - Any new bugs or gotchas discovered

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

## Contact

If you encounter issues, document them in `CRITICAL_ISSUES.md` with:
- What you were trying to do
- What actually happened
- Code snippets
- Error messages
