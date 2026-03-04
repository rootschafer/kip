use dioxus::prelude::*;

use daemon::Graph;

#[derive(Props, Clone, PartialEq)]
pub struct GraphNodeContextMenuProps {
	pub graph: Signal<Graph>,
}

// ─── Context Menu Component ───────────────────────────────────

#[component]
pub fn GraphNodeContextMenu(props: GraphNodeContextMenuProps) -> Element {
	let mut graph = props.graph;
	let menu_state = graph().context_menu.clone();

	if !menu_state.visible {
		return rsx! {};
	}

	// Get node info for display
	let node_info = menu_state.node_id.as_ref().and_then(|id| {
		graph().find_node(id).map(|n| (n.label.clone(), n.kind.clone()))
	});

	let Some((node_label, node_kind)) = node_info else {
		return rsx! {};
	};

	// Context menu actions
	let is_expandable = node_kind.is_expandable();
	let is_expanded = node_kind.is_expanded();
	
	// Clone values needed for closures
	let menu_x = menu_state.x;
	let menu_y = menu_state.y;
	let menu_node_id = menu_state.node_id.clone();
	let is_selected = menu_node_id.as_ref().map(|id| graph().selected.contains(id)).unwrap_or(false);
	
	// Clone node_id for each button to avoid move issues
	let expand_node_id = menu_node_id.clone();
	let select_node_id = menu_node_id.clone();
	let sync_node_id = menu_node_id.clone();

	rsx! {
		div {
			class: "context-menu-overlay",
			onclick: move |_| {
				graph.with_mut(|g| g.context_menu.hide());
			},
			div {
				class: "graph-context-menu",
				style: "left: {menu_x}px; top: {menu_y}px;",
				onclick: move |e| e.stop_propagation(),

				// Node info header
				div { class: "context-menu-header",
					span { class: "context-menu-node-name", "{node_label}" }
				}

				// Menu items
				div { class: "context-menu-items",
					if is_expandable {
						button {
							class: "context-menu-item",
							onclick: move |_| {
								if let Some(ref id) = expand_node_id {
									let id_clone = id.clone();
									graph.with_mut(|g| {
										g.toggle_expand(&id_clone);
										g.context_menu.hide();
									});
								}
							},
							span { if is_expanded { "▼" } else { "▶" } }
							span { if is_expanded { "Collapse" } else { "Expand" } }
						}
					}

					button {
						class: "context-menu-item",
						onclick: move |_| {
							if let Some(ref id) = select_node_id {
								let id_clone = id.clone();
								graph.with_mut(|g| {
									g.toggle_select(&id_clone);
									g.context_menu.hide();
								});
							}
						},
						span { "📋" }
						span { if is_selected { "Deselect" } else { "Select" } }
					}

					button {
						class: "context-menu-item",
						onclick: move |_| {
							if let Some(ref id) = sync_node_id {
								let id_clone = id.clone();
								graph.with_mut(|g| {
									g.drag_state = daemon::DragState::CreatingEdge {
										source_id: id_clone.clone(),
										source_x: g.find_node(&id_clone).map(|n| n.center_x()).unwrap_or(0.0),
										source_y: g.find_node(&id_clone).map(|n| n.center_y()).unwrap_or(0.0),
										mouse_x: menu_x,
										mouse_y: menu_y,
									};
									g.context_menu.hide();
								});
							}
						},
						span { "🔗" }
						span { "Create Sync..." }
					}

					div { class: "context-menu-divider" }

					button {
						class: "context-menu-item",
						onclick: move |_| {
							graph.with_mut(|g| g.context_menu.hide());
						},
						span { "✕" }
						span { "Cancel" }
					}
				}
			}
		}
	}
}
