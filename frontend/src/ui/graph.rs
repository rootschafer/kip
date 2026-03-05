use dioxus::prelude::*;
use tracing::{error, info};

use crate::{
	ui::{
		container_components::*,
		file_picker::*,
		graph_context_menu::*,
		graph_edges::*,
		graph_nodes::*,
		notification::NotificationService,
	},
};
use daemon::{DbHandle, Graph, load_graph_data};
use kip_core::{ContainerView, Vec2};

// ─── Helper: Get workspace-relative mouse coordinates ─────────────────

fn get_workspace_coords(e: &MouseEvent) -> (f64, f64) {
	// Use client coordinates and subtract header offset
	// Header height: ~61px (padding 16+16 + font 17 + border 1 + spacing)
	let client_coords = e.client_coordinates();
	(client_coords.x, client_coords.y - 61.0)
}

// ─── Graph Toolbar Component ──────────────────────────────────

#[component]
pub fn GraphToolbar(
	graph: Signal<Graph>,
	containers: Vec<ContainerView>,
	review_count: i64,
	on_add_machine_click: EventHandler,
	on_container_click: EventHandler<ContainerView>,
) -> Element {
	let status_class = if review_count > 0 {
		"status-indicator error"
	} else {
		"status-indicator ok"
	};
	let status_count = review_count;

	rsx! {
		div { class: "graph-toolbar",
			div { class: status_class,
				// Show review count, scan status, or OK status
				if graph().scanning.is_some() {
					// Show scanning status
					div { class: "spinner" }
					span { class: "status-label", "{graph().scan_progress}" }
				} else if !graph().scan_progress.is_empty() {
					// Show last scan result
					span { class: "status-label ok", "{graph().scan_progress}" }
				} else if status_count > 0 {
					// Show review count
					span { class: "status-count", "{status_count}" }
					span { class: "status-label",
						if status_count == 1 {
							"issue"
						} else {
							"issues"
						}
					}
				} else {
					span { class: "status-label", "Ready" }
				}
			}

			div { class: "machine-chips",
				for container in containers.iter() {
					MachineChip {
						container: container.clone(),
						on_click: move |c: ContainerView| {
							on_container_click.call(c);
						},
					}
				}
				button {
					class: "btn-add",
					onclick: move |_| {
						on_add_machine_click.call(());
					},
					"+"
				}
			}
		}
	}
}

// ─── Add Panel State ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AddPanelState {
	Closed,
	AddMachine,
}

// ─── Main Mapping Graph Component ──────────────────────────────

#[component]
pub fn MappingGraph(
	picker: Store<PickerManager>,
	refresh_tick: u32,
	on_changed: EventHandler,
	notifs: Store<NotificationService>,
	// db: Signal<DbHandle>,
) -> Element {
	let db = use_context::<DbHandle>();

	// // Create the main graph state as a signal
	// let mut graph = use_signal(|| Graph::new());

	// Add-machine form fields
	let mut machine_name = use_signal(|| String::new());
	let mut machine_host = use_signal(|| String::new());
	let mut machine_user = use_signal(|| String::new());
	let mut add_panel = use_signal(|| AddPanelState::Closed);

	// Create the main graph state as a signal
	let mut graph = use_signal(|| Graph::new());

	// Inject JavaScript to handle right-click context menu on nodes
	// This bypasses Dioxus event limitations by listening directly in the WebView
	use_effect(move || {
		let mut graph_signal = graph;
		spawn(async move {
			// JavaScript to inject context menu handling
			let js_code = r#"
			(function() {
				// Remove any existing listener to prevent duplicates
				if (window.__kipContextMenuInstalled) {
					document.removeEventListener('contextmenu', window.__kipContextMenuHandler, true);
				}
				
				window.__kipContextMenuHandler = async function(e) {
					// Check if we clicked on a graph node
					const node = e.target.closest('.graph-node');
					if (!node) return;
					
					// Prevent default context menu
					e.preventDefault();
					e.stopPropagation();
					
					// Get node ID from the element
					const nodeId = node.getAttribute('data-node-id');
					if (!nodeId) return;
					
					// Get viewport transform
					const workspace = document.getElementById('workspace');
					const style = window.getComputedStyle(workspace);
					const transform = style.transform;
					
					// Parse transform matrix to get scale and translation
					let scaleX = 1, scaleY = 1, translateX = 0, translateY = 0;
					if (transform && transform !== 'none') {
						const matrix = transform.match(/matrix.*\((.+)\)/);
						if (matrix) {
							const values = matrix[1].split(',').map(parseFloat);
							scaleX = values[0];
							scaleY = values[3];
							translateX = values[4];
							translateY = values[5];
						}
					}
					
					// Calculate graph-space coordinates
					const rect = workspace.getBoundingClientRect();
					const graphX = (e.clientX - rect.left - translateX) / scaleX;
					const graphY = (e.clientY - rect.top - translateY) / scaleY;
					
					// Send to Rust
					dioxus.send({
						type: 'contextmenu',
						nodeId: nodeId,
						x: graphX,
						y: graphY
					});
				};
				
				document.addEventListener('contextmenu', window.__kipContextMenuHandler, true);
				window.__kipContextMenuInstalled = true;
			})();
			"#;

			// Inject the JavaScript
			let mut eval = document::eval(js_code);

			// Listen for context menu events from JavaScript
			loop {
				match eval.recv::<serde_json::Value>().await {
					Ok(msg) => {
						if let Some(event_type) = msg.get("type").and_then(|v| v.as_str()) {
							if event_type == "contextmenu" {
								if let (Some(node_id), Some(x), Some(y)) = (
									msg.get("nodeId").and_then(|v| v.as_str()),
									msg.get("x").and_then(|v| v.as_f64()),
									msg.get("y").and_then(|v| v.as_f64()),
								) {
									graph_signal.with_mut(|g| {
										g.context_menu.show(x, y, node_id.to_string());
									});
								}
							}
						}
					}
					Err(_) => break,
				}
			}
		});
	});

	// Load graph data from DB when refresh_tick changes
	let db_for_resource = db.clone();
	let loaded_data = use_resource(move || {
		let db_val = db_for_resource.clone();
		let tick = refresh_tick;
		async move {
			let _ = tick;
			load_graph_data(&db_val).await.ok()
		}
	});

	// Update graph when data is loaded (only runs when loaded_data changes)
	use_effect(move || {
		let data = loaded_data.read();
		if let Some(Some((containers, nodes, edges, review_count))) = data.as_ref() {
			graph.with_mut(|g| {
				let had_nodes = !g.nodes.is_empty();
				g.load_from_db(containers.clone(), nodes.clone(), edges.clone(), *review_count);
				// Only start simulation on initial load to prevent constant re-simulation
				if !had_nodes {
					g.start_simulation();
				}
			});
		}
	});

	// Start the simulation loop if running
	use_effect(move || {
		tracing::info!("Simulation loop started");
		spawn(async move {
			let mut tick_count = 0;
			loop {
				// Check if simulation should run before sleeping
				let sim_state = graph.with(|g| g.sim_running);

				if !sim_state {
					// Sleep longer when not running to reduce CPU usage
					tokio::time::sleep(std::time::Duration::from_millis(100)).await;
					continue;
				}

				tick_count += 1;
				if tick_count % 10 == 0 {
					// tracing::info!("Simulation loop: tick {}", tick_count);
				}

				tokio::time::sleep(std::time::Duration::from_millis(16)).await; // ~60fps when running

				let should_continue = graph.with_mut(|g| {
					// Only tick if sim is still running
					if g.sim_running {
						let result = g.tick();
						result
					} else {
						false
					}
				});

				if !should_continue {
					// tracing::info!("Simulation loop: tick {} stopped, will restart if needed", tick_count);
					// Don't break - just reset tick count and wait for sim_running to become true again
					tick_count = 0;
				}
			}
		});
	});

	// Keyboard shortcuts
	use_effect(move || {
		let graph_signal = graph;
		spawn(async move {
			// Note: Dioxus 0.7 doesn't have global keyboard listeners yet
			// This is a placeholder for future keyboard shortcut implementation
			// Common shortcuts to implement:
			// - Delete/Backspace: Delete selected nodes
			// - Ctrl+A: Select all
			// - Escape: Deselect all, close menus
			// - Space: Pan mode
			// - +/-: Zoom in/out
			// - E: Expand selected directory
			// - C: Collapse selected directory
			// - S: Start sync (edge creation mode)
			tracing::info!("Keyboard shortcuts ready (implementation pending)");
		});
	});

	rsx! {
		div { class: "graph-area",
			// Toolbar with status and machine chips
			GraphToolbar {
				graph,
				containers: graph().containers.clone(),
				review_count: graph().review_count,
				on_add_machine_click: move |_| {
					*machine_name.write() = String::new();
					*machine_host.write() = String::new();
					*machine_user.write() = String::new();
					*add_panel.write() = AddPanelState::AddMachine;
				},
				on_container_click: move |c: ContainerView| {
					if !c.connected {
						warn!("cannot add to disconnected target");
						return;
					}
					let cid = daemon::rid_string(&c.id);
					let name = c.name.clone();
					let root = c.mount_point.clone().unwrap_or_else(|| "/".to_string());
					picker.open(cid, name, std::path::PathBuf::from(root));
				},
			}

			// Workspace: free nodes + SVG edges
			div {
				id: "workspace",
				class: "workspace",
				style: "width: 100%; height: 100%; overflow: hidden;",
				// Zoom/pan handled via Alt+drag for now
				onmousedown: move |e: MouseEvent| {
					let (x, y) = get_workspace_coords(&e);
					// Start panning on workspace click (Alt+click or middle click)
					if e.data().modifiers().alt() {
						let (vp_x, vp_y) = graph.with(|g| (g.viewport_x, g.viewport_y));
						graph
							.with_mut(|g| {
								g.drag_state = daemon::DragState::Panning {
									start_x: x,
									start_y: y,
									start_viewport_x: vp_x,
									start_viewport_y: vp_y,
								};
							});
					} else if e.data().modifiers().shift() {
						graph
							.with_mut(|g| {
								g.drag_state = daemon::DragState::Lasso {
									start_x: x,
									start_y: y,
									current_x: x,
									current_y: y,
								};
							});
					} else {
						graph
							.with_mut(|g| {
								g.clear_selection();
							});
					}
				},
				onmousemove: move |e: MouseEvent| {
					let (x, y) = get_workspace_coords(&e);
					let drag_state_snapshot = graph().drag_state.clone();

					// Handle panning - 1:1 with mouse movement
					if let daemon::DragState::Panning {
						start_x,
						start_y,
						start_viewport_x,

						// Fix the node position when drag starts
						// Update fixed position during drag
						start_viewport_y,
					} = &drag_state_snapshot {
						let dx = x - start_x;
						let dy = y - start_y;
						graph
							.with_mut(|g| {
								g.set_viewport(start_viewport_x + dx, start_viewport_y + dy)
							});
						return;
					}
					match &drag_state_snapshot {
						daemon::DragState::CreatingEdge {
							source_id,
							source_x,
							source_y,
							..
						} => {
							graph
								.with_mut(|g| {
									g.drag_state = daemon::DragState::CreatingEdge {
										source_id: source_id.clone(),
										source_x: *source_x,
										source_y: *source_y,
										mouse_x: x,
										mouse_y: y,
									};
								});
						}
						daemon::DragState::Lasso { start_x, start_y, .. } => {
							graph
								.with_mut(|g| {
									g.drag_state = daemon::DragState::Lasso {
										start_x: *start_x,
										start_y: *start_y,
										current_x: x,
										current_y: y,
									};
								});
						}
						daemon::DragState::ClickPending {
							node_id,
							start_x,
							start_y,
							..
						} => {
							let distance_moved = ((x - start_x).powi(2) + (y - start_y).powi(2))
								.sqrt();
							if distance_moved > 5.0 {
								graph
									.with_mut(|g| {
										g.fix_node_position(&node_id);
										g.drag_state = daemon::DragState::Dragging {
											node_id: node_id.clone(),
											offset_x: x - start_x,
											offset_y: y - start_y,
										};
									});
							} else {
								graph
									.with_mut(|g| {
										g.drag_state = daemon::DragState::ClickPending {
											node_id: node_id.clone(),
											start_x: *start_x,
											start_y: *start_y,
											mouse_x: x,
											mouse_y: y,
										};
									});
							}
						}
						daemon::DragState::Dragging {
							node_id,
							offset_x,
							offset_y,
						} => {
							let new_x = x - offset_x;
							let new_y = y - offset_y;
							graph
								.with_mut(|g| {
									if let Some(node) = g.find_node_mut(&node_id) {
										node.fx = Some(new_x);
										node.fy = Some(new_y);
										node.position = Vec2::new(new_x, new_y);
									}
									g.drag_state = daemon::DragState::Dragging {
										node_id: node_id.clone(),
										offset_x: *offset_x,
										offset_y: *offset_y,
									};
								});
						}
						_ => {}
					}
				},
				onmouseup: {
					let db = db.clone();
					move |_| {
						let current_drag = graph().drag_state.clone();
						match current_drag {
							// Edge creation will be handled by individual node components
							// when mouseup occurs over another node

							// Check if it was actually a click (not a drag)
							// It was a click - handle expansion for directory nodes

							// Toggle expansion for expandable nodes

							// Save the final position to the database

							daemon::DragState::CreatingEdge { source_id: _, .. } => {}
							daemon::DragState::Lasso {
								start_x,
								start_y,
								current_x,
								current_y,
							} => {
								let min_x = start_x.min(current_x);
								let max_x = start_x.max(current_x);
								let min_y = start_y.min(current_y);
								let max_y = start_y.max(current_y);
								graph
									.with_mut(|g| {
										g.select_in_rect(min_x, min_y, max_x, max_y);
										g.drag_state = daemon::DragState::None;
									});
							}
							daemon::DragState::ClickPending {
								node_id,
								start_x,
								start_y,
								mouse_x,
								mouse_y,
							} => {
								let distance_moved = ((mouse_x - start_x).powi(2)
									+ (mouse_y - start_y).powi(2))
									.sqrt();
								tracing::info!(

									// Start filesystem scan for Machine/Drive nodes

									// Get mount point from containers

									// Scan filesystem asynchronously
									// Get parent node position for orbit placement

									// Release the node and restart simulation

									// Save position to DB
									"ClickPending: node={}, distance_moved={:.2}", node_id,
									distance_moved
								);
								if distance_moved < 5.0 {
									let node_info = graph()
										.find_node(&node_id)
										.map(|n| (n.kind.clone(), n.path.clone(), n.label.clone()));
									if let Some((kind, path, label)) = node_info {
										tracing::info!(
											"Click on node {}: kind={:?}, expandable={}", node_id, kind,
											kind.is_expandable()
										);
										if kind.is_expandable() {
											tracing::info!("*** EXPANDING NODE {} ***", node_id);
											let db_clone = db.clone();
											let node_id_clone = node_id.clone();
											let label_clone = label.clone();
											let path_clone = path.clone();
											let mut graph_signal = graph;
											let is_machine_or_drive = matches!(
												&kind,
												kip_core::NodeKind::Machine { .. }
												| kip_core::NodeKind::Drive { .. }
											);
											if is_machine_or_drive && path.is_empty() {
												let mount_point = graph_signal()
													.containers
													.iter()
													.find(|c| {
														daemon::rid_string(&c.id) == node_id
													})
													.and_then(|c| c.mount_point.clone());
												graph_signal
													.with_mut(|g| {
														g.start_filesystem_scan(
															&node_id,
															&label,
															mount_point.as_deref(),
														);
														g.toggle_expand(&node_id);
													});
												let mount_point = mount_point
													.unwrap_or_else(|| "/".to_string());
												let mut graph_for_success = graph_signal;
												let mut graph_for_error = graph_signal;
												let node_id_for_scan = node_id.clone();
												let graph_for_pos = graph_signal;
												let node_id_for_pos = node_id.clone();
												spawn(async move {
													let (parent_x, parent_y) = graph_for_pos
														.with(|g| {
															g.find_node(&node_id_for_pos)
																.map(|n| (n.position.x, n.position.y))
																.unwrap_or((600.0, 400.0))
														});
													tracing::info!(
														"Scanning filesystem at: {} (parent pos: {:.0}, {:.0})",
														mount_point, parent_x, parent_y
													);
													match daemon::scan_directory(
															&db_clone,
															&node_id_clone,
															&mount_point,
															parent_x,
															parent_y,
														)
														.await
													{
														Ok(nodes) => {
															let count = nodes.len();
															graph_for_success
																.with_mut(|g| {
																	g.complete_filesystem_scan(&node_id_for_scan, nodes);
																});
															tracing::info!("Scan complete: added {} nodes", count);
														}
														Err(e) => {
															error!("Filesystem scan failed: {}", e);
															graph_for_error
																.with_mut(|g| {
																	g.clear_scan_status();
																});
														}
													}
												});
											} else {
												graph_signal
													.with_mut(|g| {
														g.toggle_expand(&node_id);
													});
											}
										}
									}
								}
								graph
									.with_mut(|g| {
										g.drag_state = daemon::DragState::None;
									});
							}
							daemon::DragState::Dragging { node_id, .. } => {
								graph
									.with_mut(|g| {
										g.release_node_position(&node_id);
										g.start_simulation();
									});
								if let Some(node) = graph().find_node(&node_id) {
									let db_clone = db.clone();
									let node_id_clone = node_id.clone();
									let x = node.position.x;
									let y = node.position.y;
									spawn(async move {
										if let Err(e) = daemon::save_node_position(
												&db_clone,
												&node_id_clone,
												x,
												y,
											)
											.await
										{
											error!("Failed to save node position: {}", e);
										}
									});
								}
								graph
									.with_mut(|g| {
										g.drag_state = daemon::DragState::None;
									});
							}
							_ => {
								graph
									.with_mut(|g| {
										g.drag_state = daemon::DragState::None;
									});
							}
						}
					}
				},

				// SVG overlay for edges and interactions with viewport transform
				{
					let (scale, x, y) = graph
						// tracing::info!("DEBUG: Rendering {} visible nodes, viewport=({},{}) scale={}", visible_count, x, y, scale);
						// Render visible nodes with viewport
						.with(|g| (g.viewport_scale, g.viewport_x, g.viewport_y));
					let visible_count = graph().visible_nodes().len();
					rsx! {
					div { style: "transform: translate({x}px, {y}px) scale({scale}); transform-origin: 0 0; width: 100%; height: 100%; position: relative;",
						GraphSvgOverlay {
							graph,
							canvas_width: 2000.0,
							canvas_height: 2000.0,
							viewport_scale: scale,
							viewport_x: x,
							viewport_y: y,
						}
						for node in graph().visible_nodes().iter() {
							GraphNodeComponent { graph, node: (*node).clone() }
						}
					}
				}
				}
				// Context menu (rendered outside viewport transform so it stays fixed on screen)
				GraphNodeContextMenu { graph }
			}

			// Add machine panel
			if *add_panel.read() == AddPanelState::AddMachine {
				div {
					class: "add-panel-overlay",
					onclick: move |_| *add_panel.write() = AddPanelState::Closed,
					div {
						class: "add-panel",
						onclick: move |e: MouseEvent| e.stop_propagation(),
						div { class: "add-panel-title", "Add remote machine" }
						div { class: "add-machine-form",
							div { class: "form-field",
								label { "Name" }
								input {
									value: "{machine_name}",
									placeholder: "My Server",
									oninput: move |e| *machine_name.write() = e.value(),
								}
							}
							div { class: "form-field",
								label { "Hostname" }
								input {
									value: "{machine_host}",
									placeholder: "192.168.1.100 or server.local",
									oninput: move |e| *machine_host.write() = e.value(),
								}
							}
							div { class: "form-field",
								label { "SSH User" }
								input {
									value: "{machine_user}",
									placeholder: "root",
									oninput: move |e| *machine_user.write() = e.value(),
								}
							}
							div { class: "form-actions-row",
								button {
									class: "btn-ghost",
									onclick: move |_| *add_panel.write() = AddPanelState::Closed,
									"Cancel"
								}
								button {
									class: "btn-primary",
									disabled: machine_host().trim().is_empty(),
									onclick: {
										let db = db.clone();
										move |_| {
											let name = machine_name().trim().to_string();
											let host = machine_host().trim().to_string();
											let user = machine_user().trim().to_string();
											let db = db.clone();
											let on_changed = on_changed;
											let mut add_panel = add_panel;
											spawn(async move {
												match daemon::add_remote_machine(
														&db,
														&name,
														&host,
														&user,
													)
													.await
												{
													Ok(()) => {
														info!("remote machine added: {}", host);
														on_changed.call(());
													}
													Err(e) => error!("add machine failed: {}", e),
												}
												*add_panel.write() = AddPanelState::Closed;
											});
										}
									},
									"Add"
								}
							}
						}
					}
				}
			}
		}
	}
}
