use dioxus::prelude::*;
use crate::ui::graph_types::{ContainerView, NodeView};
use crate::ui::graph::{rid_string, DragState};
use crate::db::DbHandle;
use tracing::{info, error};
use std::collections::{HashSet, HashMap};

#[component]
pub fn ContainerHeader(container: ContainerView) -> Element {
    let _disconnected_class = if container.connected { "" } else { " disconnected" };
    let kind_label = if container.connected {
        container.kind.as_str()
    } else {
        "offline"
    };
    
    rsx! {
        div { class: "container-header",
            div {
                class: "container-dot",
                style: "background: {container.color};",
            }
            span { class: "container-name", "{container.name}" }
            span { class: "container-kind", "{kind_label}" }
        }
    }
}

#[component]
pub fn ContainerNodes(
    container: ContainerView,
    nodes: Vec<NodeView>,
    selected: Signal<HashSet<String>>,
    drag: Signal<DragState>,
    expansion_state: Signal<HashMap<String, (bool, bool)>>,
    db: DbHandle,
    on_changed: EventHandler<()>
) -> Element {
    let cid = rid_string(&container.id);
    let container_nodes: Vec<&NodeView> = nodes
        .iter()
        .filter(|n| n.container_id == cid)
        .collect();

    rsx! {
        div { class: "container-nodes",
            for node in container_nodes.iter() {
                {
                    let node_id_str = rid_string(&node.id);
                    let node_cx = node.center_x();
                    let node_cy = node.center_y();
                    let db = db.clone();
                    let on_changed = on_changed;
                    let is_selected = selected().contains(&node_id_str);
                    let is_dir = node.is_dir;
                    let child_count_display = node.child_count.to_string();
                    let label = node.label.clone();

                    // Get expansion state for directories
                    let (is_orbit, is_expanded) = if is_dir {
                        expansion_state()
                            .get(&node_id_str)
                            .copied()
                            .unwrap_or((false, false))
                    } else {
                        (false, false)
                    };

                    let node_class = if is_dir {
                        match (is_selected, is_orbit || is_expanded) {
                            (false, false) => "graph-node dir-node",
                            (true, false) => "graph-node dir-node selected",
                            (false, true) => "graph-node dir-node expanded",
                            (true, true) => "graph-node dir-node selected expanded",
                        }
                    } else {
                        match (node.depth > 0, is_selected) {
                            (false, false) => "graph-node",
                            (true, false) => "graph-node nested",
                            (false, true) => "graph-node selected",
                            (true, true) => "graph-node nested selected",
                        }
                    };

                    rsx! {
                        div {
                            key: "{node_id_str}",
                            class: "{node_class}",

                            onmousedown: {
                                let node_id_str = node_id_str.clone();
                                move |e: MouseEvent| {
                                    e.stop_propagation();
                                    if e.modifiers().shift() {
                                        let mut sel = selected.write();
                                        if sel.contains(&node_id_str) {
                                            sel.remove(&node_id_str);
                                        } else {
                                            sel.insert(node_id_str.clone());
                                        }
                                        *drag.write() = DragState::None;
                                    } else {
                                        let coords = e.page_coordinates();
                                        *drag.write() = DragState::CreatingEdge {
                                            source_id: node_id_str.clone(),
                                            source_x: node_cx,
                                            source_y: node_cy,
                                            mouse_x: coords.x,
                                            mouse_y: coords.y,
                                        };
                                    }
                                }
                            },
                            onmouseup: {
                                let node_id_str = node_id_str.clone();
                                move |e: MouseEvent| {
                                    e.stop_propagation();
                                    let current = drag.read().clone();
                                    if let DragState::CreatingEdge { source_id, .. } = current {
                                        if source_id != node_id_str {
                                            info!("creating edge: {} -> {}", source_id, node_id_str);
                                            let source = source_id;
                                            let dest = node_id_str.clone();
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
                                let node_id = node_id_str.clone();
                                move |e: MouseEvent| {
                                    if !is_dir || e.modifiers().shift() { return; }
                                    e.stop_propagation();
                                    // Toggle: collapsed -> orbit -> expanded -> collapsed
                                    let mut state = expansion_state.write();
                                    let current = state.get(&node_id).copied().unwrap_or((false, false));
                                    let next = match current {
                                        (false, false) => (true, false),
                                        (true, false) => (false, true),
                                        _ => (false, false),
                                    };
                                    state.insert(node_id.clone(), next);
                                }
                            },

                            // Directory: show child count + label
                            if is_dir {
                                span { class: "child-count", "{child_count_display}" }
                            }
                            span { class: "node-label", "{label}" }
                            div { class: "node-handle" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn GraphContainer(
    container: ContainerView, 
    nodes: Vec<NodeView>,
    selected: Signal<HashSet<String>>,
    drag: Signal<DragState>,
    expansion_state: Signal<HashMap<String, (bool, bool)>>,
    db: DbHandle,
    on_changed: EventHandler<()>
) -> Element {
    let cid = rid_string(&container.id);
    let disconnected_class = if container.connected { "" } else { " disconnected" };
    let left = container.x;
    let top = container.y;
    let container_for_header = container.clone();

    rsx! {
        div {
            key: "{cid}",
            class: "graph-container{disconnected_class}",
            style: "left: {left}px; top: {top}px;",
            ContainerHeader { container: container_for_header }
            ContainerNodes {
                container,
                nodes,
                selected,
                drag,
                expansion_state,
                db,
                on_changed,
            }
        }
    }
}

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