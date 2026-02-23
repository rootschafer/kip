use dioxus::prelude::*;

use crate::ui::{
	graph_store::{DragState, Graph},
	graph_types::*,
};

// ─── GraphNodeComponent ────────────────────────────────────────

#[component]
pub fn GraphNodeComponent(graph: Signal<Graph>, node: GraphNode) -> Element {
	match &node.kind {
		NodeKind::File => rsx! { FileNode { graph, node } },
		NodeKind::Directory { .. } => rsx! { DirNode { graph, node } },
		NodeKind::Group { .. } => rsx! { GroupNode { graph, node } },
		NodeKind::Machine { .. } => rsx! { MachineNode { graph, node } },
		NodeKind::Drive { .. } => rsx! { DriveNode { graph, node } },
	}
}

// ─── FileNode ──────────────────────────────────────────────────

#[component]
pub fn FileNode(graph: Signal<Graph>, node: GraphNode) -> Element {
	let node_id = node.id.clone();
	let node_id_mousedown = node_id.clone();
	let node_id_mouseup = node_id.clone();
	let label = node.label.clone();
	let color = node.color.clone();
	let x = node.position.x;
	let y = node.position.y;
	let width = node.width;
	let height = node.height;
	let is_selected = graph().selected.contains(&node_id);

	let class = if is_selected { "graph-node file-node selected" } else { "graph-node file-node" };

	rsx! {
		div {
			class: "{class}",
			style: "left: {x}px; top: {y}px; width: {width}px; height: {height}px; --node-color: {color};",
			onmousedown: move |e: MouseEvent| {
				e.stop_propagation();
				let coords = e.client_coordinates();
				let (screen_x, screen_y) = (coords.x, coords.y);
				let (viewport_x, viewport_y, viewport_scale) = graph.with(|g| (g.viewport_x, g.viewport_y, g.viewport_scale));
				let mx = (screen_x - viewport_x) / viewport_scale;
				let my = (screen_y - 61.0 - viewport_y) / viewport_scale;
				let node_id_for_drag = node_id_mousedown.clone();
				if e.modifiers().shift() {
					graph.with_mut(|g| g.toggle_select(&node_id_for_drag));
				} else if e.modifiers().ctrl() || e.modifiers().alt() {
					let center_x = x + width / 2.0;
					let center_y = y + height / 2.0;
					graph.with_mut(|g| {
						g.drag_state = DragState::CreatingEdge {
							source_id: node_id_for_drag.clone(),
							source_x: center_x,
							source_y: center_y,
							mouse_x: mx,
							mouse_y: my,
						};
					});
				} else {
					graph.with_mut(|g| {
						g.drag_state = DragState::ClickPending {
							node_id: node_id_for_drag.clone(),
							start_x: mx,
							start_y: my,
							mouse_x: mx,
							mouse_y: my,
						};
					});
				}
			},
			onmouseup: move |e: MouseEvent| {
				e.stop_propagation();
				let drag_state = graph().drag_state.clone();
				match &drag_state {
					DragState::CreatingEdge { source_id, .. } => {
						if source_id != &node_id_mouseup {
							// Edge creation handled in graph.rs workspace onmouseup
						}
					}
					DragState::Dragging { .. } => {
						// Release the node - handled by workspace, but we need to reset drag state
						graph.with_mut(|g| {
							g.release_node_position(&node_id_mouseup);
							g.release_selected_nodes();
							g.start_simulation();
							g.drag_state = DragState::None;
						});
					}
					_ => {}
				}
			},
			span { class: "node-label", "{label}" }
		}
	}
}

// ─── DirNode ───────────────────────────────────────────────────

#[component]
pub fn DirNode(graph: Signal<Graph>, node: GraphNode) -> Element {
	let node_id = node.id.clone();
	let node_id_mousedown = node_id.clone();
	let node_id_mouseup = node_id.clone();
	let label = node.label.clone();
	let color = node.color.clone();
	let x = node.position.x;
	let y = node.position.y;
	let width = node.width;
	let height = node.height;
	let is_selected = graph().selected.contains(&node_id);
	let is_expanded = node.kind.is_expanded();

	let class = if is_selected { "graph-node dir-node selected" } else { "graph-node dir-node" };

	rsx! {
		div {
			class: "{class}",
			style: "left: {x}px; top: {y}px; width: {width}px; height: {height}px; --node-color: {color};",
			onmousedown: move |e: MouseEvent| {
				e.stop_propagation();
				let coords = e.client_coordinates();
				let (screen_x, screen_y) = (coords.x, coords.y);
				let (viewport_x, viewport_y, viewport_scale) = graph.with(|g| (g.viewport_x, g.viewport_y, g.viewport_scale));
				let mx = (screen_x - viewport_x) / viewport_scale;
				let my = (screen_y - 61.0 - viewport_y) / viewport_scale;
				let node_id_for_drag = node_id_mousedown.clone();
				if e.modifiers().shift() {
					graph.with_mut(|g| g.toggle_select(&node_id_for_drag));
				} else if e.modifiers().ctrl() || e.modifiers().alt() {
					let center_x = x + width / 2.0;
					let center_y = y + height / 2.0;
					graph.with_mut(|g| {
						g.drag_state = DragState::CreatingEdge {
							source_id: node_id_for_drag.clone(),
							source_x: center_x,
							source_y: center_y,
							mouse_x: mx,
							mouse_y: my,
						};
					});
				} else {
					graph.with_mut(|g| {
						g.drag_state = DragState::ClickPending {
							node_id: node_id_for_drag.clone(),
							start_x: mx,
							start_y: my,
							mouse_x: mx,
							mouse_y: my,
						};
					});
				}
			},
			onmouseup: move |e: MouseEvent| {
				e.stop_propagation();
				let drag_state = graph().drag_state.clone();
				match &drag_state {
					DragState::CreatingEdge { source_id, .. } => {
						if source_id != &node_id_mouseup {
							// Edge creation handled in graph.rs workspace onmouseup
						}
					}
					DragState::Dragging { .. } => {
						graph.with_mut(|g| {
							g.release_node_position(&node_id_mouseup);
							g.release_selected_nodes();
							g.start_simulation();
							g.drag_state = DragState::None;
						});
					}
					_ => {}
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

#[component]
pub fn GroupNode(graph: Signal<Graph>, node: GraphNode) -> Element {
	let node_id = node.id.clone();
	let node_id_mousedown = node_id.clone();
	let node_id_mouseup = node_id.clone();
	let label = node.label.clone();
	let color = node.color.clone();
	let x = node.position.x;
	let y = node.position.y;
	let width = node.width;
	let height = node.height;
	let is_selected = graph().selected.contains(&node_id);

	let class = if is_selected { "graph-node group-node selected" } else { "graph-node group-node" };

	rsx! {
		div {
			class: "{class}",
			style: "left: {x}px; top: {y}px; width: {width}px; height: {height}px; --node-color: {color};",
			onmousedown: move |e: MouseEvent| {
				e.stop_propagation();
				let coords = e.client_coordinates();
				let (screen_x, screen_y) = (coords.x, coords.y);
				let (viewport_x, viewport_y, viewport_scale) = graph.with(|g| (g.viewport_x, g.viewport_y, g.viewport_scale));
				let mx = (screen_x - viewport_x) / viewport_scale;
				let my = (screen_y - 61.0 - viewport_y) / viewport_scale;
				let node_id_for_drag = node_id_mousedown.clone();
				if e.modifiers().shift() {
					graph.with_mut(|g| g.toggle_select(&node_id_for_drag));
				} else if e.modifiers().ctrl() || e.modifiers().alt() {
					let center_x = x + width / 2.0;
					let center_y = y + height / 2.0;
					graph.with_mut(|g| {
						g.drag_state = DragState::CreatingEdge {
							source_id: node_id_for_drag.clone(),
							source_x: center_x,
							source_y: center_y,
							mouse_x: mx,
							mouse_y: my,
						};
					});
				} else {
					graph.with_mut(|g| {
						g.drag_state = DragState::ClickPending {
							node_id: node_id_for_drag.clone(),
							start_x: mx,
							start_y: my,
							mouse_x: mx,
							mouse_y: my,
						};
					});
				}
			},
			onmouseup: move |e: MouseEvent| {
				e.stop_propagation();
				let drag_state = graph().drag_state.clone();
				match &drag_state {
					DragState::CreatingEdge { source_id, .. } => {
						if source_id != &node_id_mouseup {
							// Edge creation handled in graph.rs workspace onmouseup
						}
					}
					DragState::Dragging { .. } => {
						graph.with_mut(|g| {
							g.release_node_position(&node_id_mouseup);
							g.release_selected_nodes();
							g.start_simulation();
							g.drag_state = DragState::None;
						});
					}
					_ => {}
				}
			},
			div { class: "node-content",
				span { class: "node-label", "{label}" }
			}
		}
	}
}

// ─── MachineNode ───────────────────────────────────────────────

#[component]
pub fn MachineNode(graph: Signal<Graph>, node: GraphNode) -> Element {
	let node_id = node.id.clone();
	let node_id_mousedown = node_id.clone();
	let node_id_mouseup = node_id.clone();
	let label = node.label.clone();
	let color = node.color.clone();
	let x = node.position.x;
	let y = node.position.y;
	let width = node.width;
	let height = node.height;
	let is_selected = graph().selected.contains(&node_id);

	let class = if is_selected { "graph-node machine-node selected" } else { "graph-node machine-node" };

	rsx! {
		div {
			class: "{class}",
			style: "left: {x}px; top: {y}px; width: {width}px; height: {height}px; --node-color: {color};",
			onmousedown: move |e: MouseEvent| {
				e.stop_propagation();
				let coords = e.client_coordinates();
				let (screen_x, screen_y) = (coords.x, coords.y);
				let (viewport_x, viewport_y, viewport_scale) = graph.with(|g| (g.viewport_x, g.viewport_y, g.viewport_scale));
				let mx = (screen_x - viewport_x) / viewport_scale;
				let my = (screen_y - 61.0 - viewport_y) / viewport_scale;
				let node_id_for_drag = node_id_mousedown.clone();
				if e.modifiers().shift() {
					graph.with_mut(|g| g.toggle_select(&node_id_for_drag));
				} else if e.modifiers().ctrl() || e.modifiers().alt() {
					let center_x = x + width / 2.0;
					let center_y = y + height / 2.0;
					graph.with_mut(|g| {
						g.drag_state = DragState::CreatingEdge {
							source_id: node_id_for_drag.clone(),
							source_x: center_x,
							source_y: center_y,
							mouse_x: mx,
							mouse_y: my,
						};
					});
				} else {
					graph.with_mut(|g| {
						g.drag_state = DragState::ClickPending {
							node_id: node_id_for_drag.clone(),
							start_x: mx,
							start_y: my,
							mouse_x: mx,
							mouse_y: my,
						};
					});
				}
			},
			onmouseup: move |e: MouseEvent| {
				e.stop_propagation();
				let drag_state = graph().drag_state.clone();
				match &drag_state {
					DragState::CreatingEdge { source_id, .. } => {
						if source_id != &node_id_mouseup {
							// Edge creation handled in graph.rs workspace onmouseup
						}
					}
					DragState::Dragging { .. } => {
						graph.with_mut(|g| {
							g.release_node_position(&node_id_mouseup);
							g.release_selected_nodes();
							g.start_simulation();
							g.drag_state = DragState::None;
						});
					}
					_ => {}
				}
			},
			div { class: "node-content",
				span { class: "node-label", "{label}" }
			}
		}
	}
}

// ─── DriveNode ─────────────────────────────────────────────────

#[component]
pub fn DriveNode(graph: Signal<Graph>, node: GraphNode) -> Element {
	let node_id = node.id.clone();
	let node_id_mousedown = node_id.clone();
	let node_id_mouseup = node_id.clone();
	let label = node.label.clone();
	let color = node.color.clone();
	let x = node.position.x;
	let y = node.position.y;
	let width = node.width;
	let height = node.height;
	let is_selected = graph().selected.contains(&node_id);
	let is_connected = match &node.kind {
		NodeKind::Drive { connected, .. } => *connected,
		_ => false,
	};

	let class = if is_connected {
		if is_selected { "graph-node drive-node selected" } else { "graph-node drive-node" }
	} else {
		if is_selected { "graph-node drive-node selected disconnected" } else { "graph-node drive-node disconnected" }
	};

	rsx! {
		div {
			class: "{class}",
			style: "left: {x}px; top: {y}px; width: {width}px; height: {height}px; --node-color: {color};",
			onmousedown: move |e: MouseEvent| {
				e.stop_propagation();
				let coords = e.client_coordinates();
				let (screen_x, screen_y) = (coords.x, coords.y);
				let (viewport_x, viewport_y, viewport_scale) = graph.with(|g| (g.viewport_x, g.viewport_y, g.viewport_scale));
				let mx = (screen_x - viewport_x) / viewport_scale;
				let my = (screen_y - 61.0 - viewport_y) / viewport_scale;
				let node_id_for_drag = node_id_mousedown.clone();
				if e.modifiers().shift() {
					graph.with_mut(|g| g.toggle_select(&node_id_for_drag));
				} else if e.modifiers().ctrl() || e.modifiers().alt() {
					let center_x = x + width / 2.0;
					let center_y = y + height / 2.0;
					graph.with_mut(|g| {
						g.drag_state = DragState::CreatingEdge {
							source_id: node_id_for_drag.clone(),
							source_x: center_x,
							source_y: center_y,
							mouse_x: mx,
							mouse_y: my,
						};
					});
				} else {
					graph.with_mut(|g| {
						g.drag_state = DragState::ClickPending {
							node_id: node_id_for_drag.clone(),
							start_x: mx,
							start_y: my,
							mouse_x: mx,
							mouse_y: my,
						};
					});
				}
			},
			onmouseup: move |e: MouseEvent| {
				e.stop_propagation();
				let drag_state = graph().drag_state.clone();
				match &drag_state {
					DragState::CreatingEdge { source_id, .. } => {
						if source_id != &node_id_mouseup {
							// Edge creation handled in graph.rs workspace onmouseup
						}
					}
					DragState::Dragging { .. } => {
						graph.with_mut(|g| {
							g.release_node_position(&node_id_mouseup);
							g.release_selected_nodes();
							g.start_simulation();
							g.drag_state = DragState::None;
						});
					}
					_ => {}
				}
			},
			div { class: "node-content",
				span { class: "node-label", "{label}" }
			}
		}
	}
}
