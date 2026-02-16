use dioxus::prelude::*;
use crate::ui::graph_types::*;
use crate::ui::graph_store::{Graph, DragState};

// ─── GraphNodeComponent ────────────────────────────────────────
// Main dispatcher that renders the appropriate node component based on NodeKind

#[component]
pub fn GraphNodeComponent(
    graph: Signal<Graph>,
    node: GraphNode,
) -> Element {
    match &node.kind {
        NodeKind::File => rsx! { FileNode { graph: graph, node: node } },
        NodeKind::Directory { .. } => rsx! { DirNode { graph: graph, node: node } },
        NodeKind::Group { .. } => rsx! { GroupNode { graph: graph, node: node } },
        NodeKind::Machine => rsx! { MachineNode { graph: graph, node: node } },
        NodeKind::Drive { .. } => rsx! { DriveNode { graph: graph, node: node } },
    }
}

// ─── FileNode ──────────────────────────────────────────────────
// Pill-shaped node for files

#[component]
pub fn FileNode(
    graph: Signal<Graph>,
    node: GraphNode,
) -> Element {
    let node_id = node.id.clone();
    let label = node.label.clone();
    let color = node.color.clone();
    let x = node.position.x;
    let y = node.position.y;
    let width = node.width;
    let height = node.height;
    let is_selected = graph().selected.contains(&node_id);

    let class = if is_selected {
        "graph-node file-node selected"
    } else {
        "graph-node file-node"
    };

    rsx! {
        div {
            class: "{class}",
            style: "
                left: {x}px; 
                top: {y}px; 
                width: {width}px; 
                height: {height}px; 
                --node-color: {color};
            ",
            onmousedown: move |e: MouseEvent| {
                e.stop_propagation();
                
                if e.modifiers().shift() {
                    // Toggle selection
                    graph.with_mut(|g| g.toggle_select(&node_id));
                } else if e.modifiers().ctrl() || e.modifiers().alt() {
                    // Start edge creation
                    let coords = e.page_coordinates();
                    let center_x = x + width / 2.0;
                    let center_y = y + height / 2.0;
                    
                    graph.with_mut(|g| {
                        g.drag_state = DragState::CreatingEdge {
                            source_id: node_id.clone(),
                            source_x: center_x,
                            source_y: center_y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    });
                } else {
                    // Left click - start potential drag or click action
                    let coords = e.page_coordinates();
                    
                    graph.with_mut(|g| {
                        g.drag_state = DragState::ClickPending {
                            node_id: node_id.clone(),
                            start_x: coords.x,
                            start_y: coords.y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    });
                }
            },
            span { class: "node-label", "{label}" }
        }
    }
}

// ─── DirNode ───────────────────────────────────────────────────
// Circle-shaped node for directories

#[component]
pub fn DirNode(
    graph: Signal<Graph>,
    node: GraphNode,
) -> Element {
    let node_id = node.id.clone();
    let label = node.label.clone();
    let color = node.color.clone();
    let x = node.position.x;
    let y = node.position.y;
    let width = node.width;
    let height = node.height;
    let is_selected = graph().selected.contains(&node_id);
    let is_expanded = node.kind.is_expanded();

    let class = if is_selected {
        "graph-node dir-node selected"
    } else {
        "graph-node dir-node"
    };

    rsx! {
        div {
            class: "{class}",
            style: "
                left: {x}px; 
                top: {y}px; 
                width: {width}px; 
                height: {height}px; 
                --node-color: {color};
            ",
            onmousedown: move |e: MouseEvent| {
                e.stop_propagation();
                
                if e.modifiers().shift() {
                    // Toggle selection
                    graph.with_mut(|g| g.toggle_select(&node_id));
                } else if e.modifiers().ctrl() || e.modifiers().alt() {
                    // Start edge creation
                    let coords = e.page_coordinates();
                    let center_x = x + width / 2.0;
                    let center_y = y + height / 2.0;
                    
                    graph.with_mut(|g| {
                        g.drag_state = DragState::CreatingEdge {
                            source_id: node_id.clone(),
                            source_x: center_x,
                            source_y: center_y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    });
                } else {
                    // Left click - toggle expansion for directories
                    let coords = e.page_coordinates();
                    
                    graph.with_mut(|g| {
                        g.drag_state = DragState::ClickPending {
                            node_id: node_id.clone(),
                            start_x: coords.x,
                            start_y: coords.y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    });
                }
            },
            div { class: "node-content",
                span { class: "node-label", "{label}" }
                if is_expanded {
                    span { class: "expansion-indicator", "▼" }
                } else {
                    span { class: "expansion-indicator", "▶" }
                }
            }
        }
    }
}

// ─── GroupNode ─────────────────────────────────────────────────
// Circle-shaped node for groups

#[component]
pub fn GroupNode(
    graph: Signal<Graph>,
    node: GraphNode,
) -> Element {
    let node_id = node.id.clone();
    let label = node.label.clone();
    let color = node.color.clone();
    let x = node.position.x;
    let y = node.position.y;
    let width = node.width;
    let height = node.height;
    let is_selected = graph().selected.contains(&node_id);

    let class = if is_selected {
        "graph-node group-node selected"
    } else {
        "graph-node group-node"
    };

    rsx! {
        div {
            class: "{class}",
            style: "
                left: {x}px; 
                top: {y}px; 
                width: {width}px; 
                height: {height}px; 
                --node-color: {color};
            ",
            onmousedown: move |e: MouseEvent| {
                e.stop_propagation();
                
                if e.modifiers().shift() {
                    // Toggle selection
                    graph.with_mut(|g| g.toggle_select(&node_id));
                } else if e.modifiers().ctrl() || e.modifiers().alt() {
                    // Start edge creation
                    let coords = e.page_coordinates();
                    let center_x = x + width / 2.0;
                    let center_y = y + height / 2.0;
                    
                    graph.with_mut(|g| {
                        g.drag_state = DragState::CreatingEdge {
                            source_id: node_id.clone(),
                            source_x: center_x,
                            source_y: center_y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    });
                } else {
                    // Left click - start potential drag or click action
                    let coords = e.page_coordinates();
                    
                    graph.with_mut(|g| {
                        g.drag_state = DragState::ClickPending {
                            node_id: node_id.clone(),
                            start_x: coords.x,
                            start_y: coords.y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    });
                }
            },
            div { class: "node-content",
                span { class: "node-label", "{label}" }
            }
        }
    }
}

// ─── MachineNode ───────────────────────────────────────────────
// Circle-shaped node for machines

#[component]
pub fn MachineNode(
    graph: Signal<Graph>,
    node: GraphNode,
) -> Element {
    let node_id = node.id.clone();
    let label = node.label.clone();
    let color = node.color.clone();
    let x = node.position.x;
    let y = node.position.y;
    let width = node.width;
    let height = node.height;
    let is_selected = graph().selected.contains(&node_id);

    let class = if is_selected {
        "graph-node machine-node selected"
    } else {
        "graph-node machine-node"
    };

    rsx! {
        div {
            class: "{class}",
            style: "
                left: {x}px; 
                top: {y}px; 
                width: {width}px; 
                height: {height}px; 
                --node-color: {color};
            ",
            onmousedown: move |e: MouseEvent| {
                e.stop_propagation();
                
                if e.modifiers().shift() {
                    // Toggle selection
                    graph.with_mut(|g| g.toggle_select(&node_id));
                } else if e.modifiers().ctrl() || e.modifiers().alt() {
                    // Start edge creation
                    let coords = e.page_coordinates();
                    let center_x = x + width / 2.0;
                    let center_y = y + height / 2.0;
                    
                    graph.with_mut(|g| {
                        g.drag_state = DragState::CreatingEdge {
                            source_id: node_id.clone(),
                            source_x: center_x,
                            source_y: center_y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    });
                } else {
                    // Left click - start potential drag or click action
                    let coords = e.page_coordinates();
                    
                    graph.with_mut(|g| {
                        g.drag_state = DragState::ClickPending {
                            node_id: node_id.clone(),
                            start_x: coords.x,
                            start_y: coords.y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    });
                }
            },
            div { class: "node-content",
                span { class: "node-label", "{label}" }
            }
        }
    }
}

// ─── DriveNode ─────────────────────────────────────────────────
// Circle-shaped node for drives

#[component]
pub fn DriveNode(
    graph: Signal<Graph>,
    node: GraphNode,
) -> Element {
    let node_id = node.id.clone();
    let label = node.label.clone();
    let color = node.color.clone();
    let x = node.position.x;
    let y = node.position.y;
    let width = node.width;
    let height = node.height;
    let is_selected = graph().selected.contains(&node_id);
    let is_connected = match &node.kind {
        NodeKind::Drive { connected } => *connected,
        _ => false,
    };

    let class = if is_connected {
        if is_selected {
            "graph-node drive-node selected"
        } else {
            "graph-node drive-node"
        }
    } else {
        if is_selected {
            "graph-node drive-node selected disconnected"
        } else {
            "graph-node drive-node disconnected"
        }
    };

    rsx! {
        div {
            class: "{class}",
            style: "
                left: {x}px; 
                top: {y}px; 
                width: {width}px; 
                height: {height}px; 
                --node-color: {color};
            ",
            onmousedown: move |e: MouseEvent| {
                e.stop_propagation();
                
                if e.modifiers().shift() {
                    // Toggle selection
                    graph.with_mut(|g| g.toggle_select(&node_id));
                } else if e.modifiers().ctrl() || e.modifiers().alt() {
                    // Start edge creation
                    let coords = e.page_coordinates();
                    let center_x = x + width / 2.0;
                    let center_y = y + height / 2.0;
                    
                    graph.with_mut(|g| {
                        g.drag_state = DragState::CreatingEdge {
                            source_id: node_id.clone(),
                            source_x: center_x,
                            source_y: center_y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    });
                } else {
                    // Left click - start potential drag or click action
                    let coords = e.page_coordinates();
                    
                    graph.with_mut(|g| {
                        g.drag_state = DragState::ClickPending {
                            node_id: node_id.clone(),
                            start_x: coords.x,
                            start_y: coords.y,
                            mouse_x: coords.x,
                            mouse_y: coords.y,
                        };
                    });
                }
            },
            div { class: "node-content",
                span { class: "node-label", "{label}" }
            }
        }
    }
}