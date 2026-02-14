use dioxus::prelude::*;
use crate::ui::graph_types::{ContainerView, NodeView};
use crate::ui::graph::{rid_string, DragState};
use crate::db::DbHandle;
use tracing::{info, error};
use std::collections::{HashSet, HashMap};

// ─── MachineChip ─────────────────────────────────────────────
// Toolbar button representing a machine or drive.
// Clicking opens the file picker for that target.

#[component]
pub fn MachineChip(
    container: ContainerView,
    on_click: EventHandler<ContainerView>,
) -> Element {
    let name = container.name.clone();
    let color = container.color.clone();
    let connected = container.connected;
    let kind_label = if connected {
        container.kind.as_str()
    } else {
        "offline"
    };
    let opacity = if connected { "1" } else { "0.5" };

    rsx! {
        button {
            class: "machine-chip",
            style: "opacity: {opacity};",
            disabled: !connected,
            onclick: move |_| on_click.call(container.clone()),
            div { class: "chip-dot", style: "background: {color};" }
            span { class: "chip-name", "{name}" }
            span { class: "chip-kind", "{kind_label}" }
        }
    }
}

// ─── WorkspaceNode ───────────────────────────────────────────
// A single node freely positioned in the workspace.
// Directories render as circles, files as pills.
// Color-tinted by parent machine/drive.

#[component]
pub fn WorkspaceNode(
    node: NodeView,
    color: String,
    selected: Signal<HashSet<String>>,
    drag: Signal<DragState>,
    expansion_state: Signal<HashMap<String, (bool, bool)>>,
    db: DbHandle,
    on_changed: EventHandler<()>,
) -> Element {
    let node_id = rid_string(&node.id);
    let is_selected = selected().contains(&node_id);
    let is_dir = node.is_dir;
    let x = node.x;
    let y = node.y;
    let cx = node.center_x();
    let cy = node.center_y();
    let label = node.label.clone();
    let _child_count = node.child_count.to_string();  // Keeping for potential future use
    let total_descendants = node.total_descendants;

    // Get expansion state for directories
    let (is_orbit, is_expanded) = if is_dir {
        expansion_state()
            .get(&node_id)
            .copied()
            .unwrap_or((false, false))
    } else {
        (false, false)
    };

    // Calculate size based on total descendants for directory nodes
    let size_style = if is_dir {
        let _base_size = 56.0; // Default size for directories
        let size_factor = (1.0 + (total_descendants as f64).ln() * 5.0).min(120.0).max(30.0);
        format!("width: {}px; height: {}px;", size_factor, size_factor)
    } else {
        format!("width: {}px; height: {}px;", node.width, node.height)
    };

    // Determine the class based on node type and expansion state
    let base_class = if is_dir {
        if is_selected { "ws-node ws-dir selected" } else { "ws-node ws-dir" }
    } else {
        if is_selected { "ws-node ws-file selected" } else { "ws-node ws-file" }
    };
    
    // Add expansion state classes
    let class = if is_orbit && is_expanded {
        format!("{} orbit expanded", base_class)
    } else if is_orbit {
        format!("{} orbit", base_class)
    } else if is_expanded {
        format!("{} expanded", base_class)
    } else {
        base_class.to_string()
    };

    rsx! {
        div {
            key: "{node_id}",
            class: "{class}",
            style: "left: {x}px; top: {y}px; --node-color: {color}; {size_style}",

            onmousedown: {
                let node_id = node_id.clone();
                move |e: MouseEvent| {
                    e.stop_propagation();
                    if e.modifiers().shift() {
                        let mut sel = selected.write();
                        if sel.contains(&node_id) {
                            sel.remove(&node_id);
                        } else {
                            sel.insert(node_id.clone());
                        }
                        *drag.write() = DragState::None;
                    } else if e.modifiers().ctrl() || e.modifiers().alt() {  // Use Ctrl+click or Alt+click for edge creation
                        let coords = e.page_coordinates();
                        *drag.write() = DragState::CreatingEdge {
                            source_id: node_id.clone(),
                            source_x: cx,
                            source_y: cy,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    } else {  // Left-click: Start expansion sequence (orbit view)
                        // Don't start edge creation on left-click anymore
                        // Expansion will be handled on mouseup if it's a click (small movement)
                        let coords = e.page_coordinates();
                        *drag.write() = DragState::LeftClickPending {
                            node_id: node_id.clone(),
                            start_x: coords.x,
                            start_y: coords.y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    }
                }
            },
            onmousemove: move |e: MouseEvent| {
                let current_drag = drag.read().clone();
                let coords = e.page_coordinates();
                match current_drag {
                    DragState::LeftClickPending { node_id, start_x, start_y, .. } => {
                        // Update mouse position for potential drag
                        let distance_moved = ((coords.x - start_x).powi(2) + (coords.y - start_y).powi(2)).sqrt();
                        if distance_moved > 5.0 {
                            // Convert to drag state if moved significantly
                            *drag.write() = DragState::LeftDragging {
                                node_id,
                                start_x,
                                start_y,
                                mouse_x: coords.x,
                                mouse_y: coords.y,
                            };
                        } else {
                            // Still pending click
                            *drag.write() = DragState::LeftClickPending {
                                node_id,
                                start_x,
                                start_y,
                                mouse_x: coords.x,
                                mouse_y: coords.y,
                            };
                        }
                    }
                    DragState::LeftDragging { node_id, start_x, start_y, .. } => {
                        // Update drag position
                        *drag.write() = DragState::LeftDragging {
                            node_id,
                            start_x,
                            start_y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    }
                    DragState::CreatingEdge { source_id, source_x, source_y, .. } => {
                        // Update edge creation drag
                        *drag.write() = DragState::CreatingEdge {
                            source_id,
                            source_x,
                            source_y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    }
                    _ => {}
                }
            },
            onmouseup: {
                let node_id = node_id.clone();
                let db = db.clone();
                move |e: MouseEvent| {
                    e.stop_propagation();
                    let current = drag.read().clone();
                    match current {
                        DragState::LeftClickPending { node_id: click_node_id, start_x, start_y, mouse_x, mouse_y } => {
                            // This was a left-click that didn't move much - handle as expansion
                            if click_node_id == node_id && is_dir {
                                let distance_moved = ((mouse_x - start_x).powi(2) + (mouse_y - start_y).powi(2)).sqrt();
                                if distance_moved < 5.0 {
                                    // Toggle expansion state on left-click
                                    let mut state = expansion_state.write();
                                    let current = state.get(&node_id).copied().unwrap_or((false, false));
                                    let next = match current {
                                        (false, false) => (true, false),  // Enter orbit state (single click)
                                        (true, false) => (false, true),  // Enter expanded state
                                        (false, true) => (false, false), // Exit expanded state
                                        _ => (false, false),
                                    };
                                    state.insert(node_id.clone(), next);
                                }
                            }
                            *drag.write() = DragState::None;
                        }
                        DragState::LeftDragging { node_id: drag_node_id, start_x, start_y, mouse_x, mouse_y } => {
                            // This was a left-drag - treat as regular drag
                            if drag_node_id != node_id {
                                // Create edge if dropped on different node
                                info!("creating edge: {} -> {}", drag_node_id, node_id);
                                let source = drag_node_id;
                                let dest = node_id.clone();
                                let db = db.clone();
                                let on_changed = on_changed;
                                spawn(async move {
                                    match create_edge(&db, &source, &dest).await {
                                        Ok(()) => info!("edge created"),
                                        Err(e) => error!("edge creation failed: {}", e),
                                    }
                                    on_changed.call(());
                                });
                            }
                            *drag.write() = DragState::None;
                        }
                        DragState::CreatingEdge { source_id, source_x, source_y, mouse_x, mouse_y } => {
                            // Right-click drag for edge creation
                            if source_id != node_id {
                                info!("creating edge: {} -> {}", source_id, node_id);
                                let source = source_id;
                                let dest = node_id.clone();
                                let db = db.clone();
                                let on_changed = on_changed;
                                spawn(async move {
                                    match create_edge(&db, &source, &dest).await {
                                        Ok(()) => info!("edge created"),
                                        Err(e) => error!("edge creation failed: {}", e),
                                    }
                                    on_changed.call(());
                                });
                            }
                            *drag.write() = DragState::None;
                        }
                        _ => {
                            *drag.write() = DragState::None;
                        }
                    }
                }
            },
            oncontextmenu: move |e: Event<MouseData>| {
                e.prevent_default(); // Prevent default context menu
                // Right-click is now for edge creation, so we don't need special handling here
            },

            // Content varies based on expansion state
            if is_expanded {
                // In expanded state, this would show the directory contents
                // For now, we'll just show the label and handle
                span { class: "node-label", "{label}" }
                NodeHandle {}
            } else {
                // Normal view (collapsed or orbit)
                if is_dir {
                    div { class: "node-info",
                        span { class: "node-label", "{label}" }
                        span { class: "total-descendants", "{total_descendants}" }
                    }
                } else {
                    span { class: "node-label", "{label}" }
                }
                NodeHandle {}
            }
        }
    }
}

// ─── NodeHandle ──────────────────────────────────────────────
// The small circle on the right edge of a node for edge creation.

#[component]
fn NodeHandle() -> Element {
    rsx! {
        div { class: "node-handle" }
    }
}

// ─── Helpers ─────────────────────────────────────────────────

async fn create_edge(db: &DbHandle, source_id: &str, dest_id: &str) -> Result<(), String> {
    let (_, src_key) = parse_rid(source_id).ok_or("Invalid source ID")?;
    let (_, dst_key) = parse_rid(dest_id).ok_or("Invalid dest ID")?;

    db.db
        .query(
            "LET $src = type::record('location', $src_key);
             LET $dst = type::record('location', $dst_key);
             CREATE intent CONTENT {
                 source: $src,
                 destinations: [$dst],
                 status: 'idle',
                 kind: 'one_shot',
                 speed_mode: 'normal',
                 priority: 0,
                 total_files: 0,
                 total_bytes: 0,
                 completed_files: 0,
                 completed_bytes: 0,
                 bidirectional: false,
                 initial_sync_complete: false,
                 created_at: time::now(),
                 updated_at: time::now(),
             }",
        )
        .bind(("src_key", src_key.to_string()))
        .bind(("dst_key", dst_key.to_string()))
        .await.map_err(|e| e.to_string())?
        .check().map_err(|e| e.to_string())?;

    Ok(())
}

fn parse_rid(s: &str) -> Option<(&str, &str)> {
    s.split_once(':')
}
