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
    let child_count = node.child_count.to_string();

    // Get expansion state for directories
    let (is_orbit, is_expanded) = if is_dir {
        expansion_state()
            .get(&node_id)
            .copied()
            .unwrap_or((false, false))
    } else {
        (false, false)
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
            style: "left: {x}px; top: {y}px; --node-color: {color};",

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
                    } else {
                        let coords = e.page_coordinates();
                        *drag.write() = DragState::CreatingEdge {
                            source_id: node_id.clone(),
                            source_x: cx,
                            source_y: cy,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    }
                }
            },
            onmouseup: {
                let node_id = node_id.clone();
                let db = db.clone();
                move |e: MouseEvent| {
                    e.stop_propagation();
                    let current = drag.read().clone();
                    if let DragState::CreatingEdge { source_id, .. } = current {
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
                    }
                    *drag.write() = DragState::None;
                }
            },
            onclick: {
                let node_id = node_id.clone();
                move |e: MouseEvent| {
                    if !is_dir || e.modifiers().shift() { return; }
                    e.stop_propagation();
                    let mut state = expansion_state.write();
                    let current = state.get(&node_id).copied().unwrap_or((false, false));
                    let next = match current {
                        (false, false) => (true, false),  // Enter orbit state
                        (true, false) => (false, true),  // Enter expanded state
                        (false, true) => (false, false), // Exit expanded state
                        _ => (false, false),
                    };
                    state.insert(node_id.clone(), next);
                }
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
                    span { class: "child-count", "{child_count}" }
                }
                span { class: "node-label", "{label}" }
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
