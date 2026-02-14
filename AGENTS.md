# Kip Technical Reference

Authoritative reference for non-obvious patterns. Read the code first — this covers the gotchas.

---

## Build Commands

**Always use `dx build` and `dx serve --platform desktop`**. Never use `cargo build` or `cargo run`. The Dioxus CLI handles asset bundling and platform-specific setup.

```sh
dx build                        # build
dx serve --platform desktop     # run with hot reload
```

---

## Dioxus 0.7 RSX Gotchas

- **No `cx`, `Scope`, or `use_state`**. Everything is signals and `#[component]` functions returning `Element`.
- **Text in RSX** must be string literals `"hello"` or format strings `"{variable}"`. No inline expressions as text — extract to a variable first.
- **Conditional rendering**: `if cond { rsx! {...} }` works, but not `if cond { "text" } else { &variable }` as text content.
- **Loops**: Prefer `for item in items { rsx!{...} }` over `.map()`. Iterators in braces also work: `{items.iter().map(|i| rsx!{...})}`.
- **Props**: Owned types only (`String` not `&str`). Must be `Clone + PartialEq`.
- **`EventHandler`**: Use `EventHandler` (not closures) for callback props. Call with `.call(value)`.
- **Assets**: `const X: Asset = asset!("/assets/file.css");` — path is relative to project root.

### CRITICAL: Never Nest `rsx!` Inside `rsx!`

In older Dioxus versions, nesting `rsx!` macros was common (e.g., `for item in items { rsx! { div { ... } } }`). **This is no longer recommended and causes subtle macro parsing bugs.** Instead, extract any non-trivial RSX block into its own `#[component]` function. This is not extra work — it is Dioxus and Rust guiding you toward optimal, maintainable state management.

**Rules:**
- Every button should be its own component. Every icon should be its own component. If a button has an icon, the icon is a subcomponent of the button.
- Create subcomponents aggressively. Small, focused components are easier to reason about, enable code reuse, and make beautiful, maintainable apps.
- Use `ReadOnlySignal<T>` for component inputs that the component will only read (not write). Use `Signal<T>` only when the component needs to mutate the value.
- Inside `for` loops in RSX, put component calls directly (e.g., `for item in items { MyComponent { prop: item.clone() } }`), never wrap in `rsx!`.
- Inside `match` arms in RSX, use `rsx! { ... }` for each arm (e.g., `Variant => rsx! { div { ... } }`).
- If you need Rust logic (let bindings, conditionals) before RSX in a loop body, use a code block: `{ let x = ...; rsx! { ... } }`. But prefer extracting to a component instead.

## Logging

`use dioxus::prelude::*` re-exports tracing macros: `info!`, `warn!`, `error!`, `debug!`, `trace!`. Use these everywhere — they go to the `dx serve` terminal and to `kip.log` (configured in `main.rs` via `tracing-appender`).

**Never show errors in the UI** unless the user needs to act on them. Log with `error!()` and show a graceful empty state instead.

---

## SurrealDB 3.0 Gotchas

These are hard-won. SurrealDB 3.0 changed a lot from 2.x.

### Engine & RecordId

- Engine: `surrealdb::engine::local::SurrealKv` (NOT RocksDb)
- RecordId: `surrealdb::types::RecordId` (NOT `surrealdb::RecordId`)
- RecordId has **no Display impl**. Debug output is `RecordId { table: Table("x"), key: String("y") }` — useless. Use the `rid_string()` helper in `graph.rs` which produces `table:key`.
- Direct `RecordId == RecordId` comparison works and is preferred over string comparison.

### Querying

- **`.take::<Vec<serde_json::Value>>()` FAILS** with "Expected any, got record" when results contain record ID fields. Use typed structs with `#[derive(SurrealValue)]` instead.
- `SurrealValue` derive requires `surrealdb-types` as a **direct** Cargo dependency (the macro references the crate by name).
- `.bind()` needs **owned values** (`String`, not `&String` or `&str`).
- `type::thing()` was renamed to `type::record()` in 3.0.
- **ORDER BY fields must appear in SELECT**: `SELECT id, name FROM x ORDER BY created_at` fails — must include `created_at` in select list.
- **`type::record()` table name must be a query literal**, not a bind parameter. Use `format!("type::record('{table}', $key)")`, not `type::record($table, $key)` with `.bind(("table", ...))`. The key can be bound.

### Schema

- **ALL `DEFINE` statements need `OVERWRITE`** to be idempotent: `DEFINE TABLE OVERWRITE`, `DEFINE FIELD OVERWRITE`, `DEFINE INDEX OVERWRITE`.
- **SCHEMAFULL nested objects**: If a field is `TYPE option<object>`, you must also define each nested field explicitly (e.g., `DEFINE FIELD OVERWRITE foo.bar ON table TYPE option<int>`).

### Runtime

- Don't drop the Tokio runtime after DB init — SurrealDB needs it for background channels. Use `Box::leak(Box::new(rt))`.
- DB path: `~/Library/Application Support/kip/kip.db`

---

## macOS I/O Priority (Ninja Mode)

```rust
extern "C" {
    fn setiopolicy_np(iotype: i32, scope: i32, policy: i32) -> i32;
}
// IOPOL_TYPE_DISK=0, IOPOL_SCOPE_THREAD=2, IOPOL_THROTTLE=3, IOPOL_DEFAULT=0, IOPOL_NORMAL=1
```

---

## Drive Detection

Uses `diskutil info -plist <path>` to get volume UUID, name, filesystem, size, internal flag. Polls `/Volumes/` every 5 seconds. Skips symlinks (boot volume) and internal drives. See `src/devices/macos.rs`.

---

## Graph UI Architecture (Current State)

### Layout Model: Free Workspace

Nodes are **NOT** inside container cards. They are **free-floating, absolutely-positioned** elements in the workspace. Machine/drive info lives in the **toolbar as chips** — clickable buttons that open the file picker.

### Component Architecture

Components live in `src/ui/container_components.rs` (being restructured):
- `MachineChip` — Toolbar button for each machine/drive, opens file picker on click
- `WorkspaceNode` — A single node freely positioned in the workspace
- Old `GraphContainer`/`ContainerHeader`/`ContainerNodes` are **DEPRECATED** and being removed

### Node Types

- **Files**: Pill-shaped (rounded rect), left border tinted with machine/drive color
- **Directories**: Circle-shaped, border tinted with machine/drive color, shows child count
- **Groups**: Circle-shaped (same as directories conceptually)
- Node color comes from the parent machine/drive palette color via `--node-color` CSS variable

### Directory/Group Interaction

- **Click once**: Orbit view — direct children fan out around the circle
- **Double-click**: Enter — workspace shows only that node's children, breadcrumb for navigation back
- Three-state expansion: collapsed → orbit → entered → collapsed
- Managed through `expansion_state: Signal<HashMap<String, (bool, bool)>>`

### Node Positioning

- Currently: simple grid layout computed in `load_nodes`
- Future: force-directed physics with container-color cohesion
- Positions will persist to SurrealDB (`graph_x`/`graph_y` fields on location)

### Data Flow

- `NodeView.container_id` links each node to its machine/drive
- Container color looked up via `color_map: HashMap<String, String>`
- `is_dir` detected via filesystem metadata + child count heuristic
- `child_count` computed from path containment between sibling locations

---

## CRITICAL: Design Documentation Reference

**Every AI agent working on this codebase must read ALL files in `./notes/the_design/` before starting work.** This directory contains the authoritative development plan and architectural decisions.

The design files in `./notes/the_design/` form a cohesive, interconnected plan that describes the current state and future direction of the project. These files should be kept up-to-date with the current development status. When implementing features, check these files first to understand the intended architecture and avoid conflicting implementations.

If you complete a feature that was documented in these files, remove or update the implementation details to prevent future AI agents from duplicating work or breaking existing functionality with outdated instructions.

The design documentation is organized by phases:
- `COMPREHENSIVE_DEVELOPMENT_PLAN.md` - Overall roadmap with cross-phase references
- `Phase1/` - Core functionality (directory expansion, grouping, layout engine)
  - `Phase1.1_Directory_Expansion_Implementation.md` - Directory expansion implementation
  - `Phase1.1_Directory_Expansion_and_File_Picker.md` - Combined directory expansion and file picker implementation
  - `Phase1.2_Node_Grouping_Implementation.md` - Node grouping functionality
  - `Phase1.3_Force_Directed_Layout_Implementation.md` - Force-directed layout engine
- `Phase2/` - Transfer engine (core transfers, intent lifecycle, error handling)
  - `Phase2.1_Core_Transfer_Engine.md` - Core file transfer functionality
  - `Phase2.2_Intent_Lifecycle_Management.md` - Intent state management (moved from KIP_DESIGN_3_INTENT_LIFECYCLE.md)
  - `Phase2.3_Error_Handling_and_Review_Queue.md` - Error handling and review queue (moved from KIP_DESIGN_5_ERROR_HANDLING.md)
- `Phase3/` - Advanced features (visualization, remote access, performance)
  - `Phase3.1_Advanced_Visualization.md` - Advanced visualization features
  - `Phase3.2_Remote_Access_and_Security.md` - Remote access and security
  - `Phase3.3_Performance_Optimization.md` - Performance optimization
- `Phase4/` - Production readiness (testing, deployment, documentation)
  - `Phase4.1_Testing_and_Quality_Assurance.md` - Testing and QA
  - `Phase4.2_Deployment_and_Distribution.md` - Deployment and distribution
  - `Phase4.3_Documentation_and_Support.md` - Documentation and support

Foundational design documents (remain in main directory):
- `KIP_DESIGN_1.md` - High-level vision and architecture
- `KIP_DESIGN_2_DATA_MODEL.md` - Data model and database schema
- `KIP_DESIGN_4_ARCHITECTURE.md` - System architecture
- `KIP_DESIGN_6_MVP.md` - MVP feature set
- `KIP_DESIGN_7_MAPPING_GRAPH.md` - Mapping graph UI design

Critical Current Issues to Address:
- **SVG Coordinate System Alignment**: Mouse coordinates don't align with SVG overlay coordinates, causing cursor offset in edge creation - See `CRITICAL_ISSUES.md` for details
- **Click vs Drag Detection**: Current implementation interferes with edge creation when trying to expand nodes - See `CRITICAL_ISSUES.md` for details
- **Orbit View Implementation**: Children not properly fanned out around parent nodes in orbit state - See `CRITICAL_ISSUES.md` for details
- **Enter View Implementation**: Workspace not properly filtered to show only direct children of entered directory - See `CRITICAL_ISSUES.md` for details

Always maintain consistency between the design documentation and the actual implementation.