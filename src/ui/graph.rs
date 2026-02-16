
use dioxus::prelude::*;
use tracing::{error, info};

use crate::db::DbHandle;
use crate::ui::file_picker::*;
use crate::ui::graph_types::*;
use crate::ui::graph_store::{Graph, load_graph_data};
use crate::ui::graph_nodes::*;
use crate::ui::graph_edges::*;
use crate::ui::notification::{NotificationService, NotificationServiceStoreImplExt};
use crate::ui::container_components::*;

// ─── Graph Toolbar Component ──────────────────────────────────

#[component]
pub fn GraphToolbar(
    graph: Signal<Graph>,
    containers: Vec<ContainerView>,
    review_count: i64,
    on_add_machine_click: EventHandler,
    on_container_click: EventHandler<ContainerView>,
) -> Element {
    let status_class = if review_count > 0 { "status-indicator error" } else { "status-indicator ok" };
    let status_count = review_count;

    rsx! {
		div { class: "graph-toolbar",
			div { class: if graph().sim_running { "status-indicator processing" } else { status_class },
				// Show spinner when loading or processing
				if graph().sim_running {
					div { class: "spinner" }
				} else if status_count > 0 {
					span { class: "status-count", "{status_count}" }
					span { class: "status-label",
						if status_count == 1 {
							"issue"
						} else {
							"issues"
						}
					}
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

    // Load graph data from DB when refresh_tick changes
    let db_for_resource = db.clone();
    let loaded_data = use_resource(move || {
        let db_val = db_for_resource.clone();
        let tick = refresh_tick; // Capture tick VALUE (u32), not the signal
        async move {
            let _ = tick; // Use tick to create dependency
            load_graph_data(&db_val).await.ok()
        }
    });

    // Update graph when data is loaded (only runs when loaded_data changes)
    use_effect(move || {
        let data = loaded_data.read();
        if let Some(Some((containers, nodes, edges, review_count))) = data.as_ref() {
            graph.with_mut(|g| {
                g.load_from_db(containers.clone(), nodes.clone(), edges.clone(), *review_count);
            });
        }
    });

    let canvas_width = 1200.0_f64;
    let canvas_height = 800.0_f64;

    // // Start the simulation loop if running
    // use_effect(move || {
    //     let graph_signal = graph;
    //     spawn(async move {
    //         loop {
    //             // Check if simulation should run before sleeping
    //             let should_run = graph_signal.with(|g| g.sim_running);
    //             if !should_run {
    //                 // Sleep longer when not running to reduce CPU usage
    //                 tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    //                 continue;
    //             }
    //
    //             tokio::time::sleep(std::time::Duration::from_millis(16)).await; // ~60fps when running
    //
    //             let should_continue = graph_signal.with_mut(|g| {
    //                 // Only tick if sim is still running
    //                 if g.sim_running {
    //                     let result = g.tick();
    //                     result
    //                 } else {
    //                     false
    //                 }
    //             });
    //
    //             if !should_continue {
    //                 // Stop the loop if simulation is not running
    //                 break;
    //             }
    //         }
    //     });
    // });


                    // // let should_continue = graph_signal.with_mut(|g| {
                    // let should_continue = graph.with_mut(|g| {
                    //     // Only tick if sim is still running
                    //     if g.sim_running {
                    //         let result = g.tick();
                    //         result
                    //     } else {
                    //         false
                    //     }
                    // });
                    //
                    // if !should_continue {
                    //     // Stop the loop if simulation is not running
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
				    let cid = crate::ui::graph_store::rid_string(&c.id);
				    let name = c.name.clone();
				    let root = c.mount_point.clone().unwrap_or_else(|| "/".to_string());
				    picker.open(cid, name, std::path::PathBuf::from(root));
				},
			}

			// Workspace: free nodes + SVG edges
			div {
				class: "workspace",
				onmousedown: move |e: MouseEvent| {
				    if e.modifiers().shift() {
				        let coords = e.page_coordinates();
				        graph
				            .with_mut(|g| {
				                g.drag_state = crate::ui::graph_store::DragState::Lasso {
				                    start_x: coords.x,
				                    start_y: coords.y,
				                    current_x: coords.x,
				                    current_y: coords.y,
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
				    let coords = e.page_coordinates();
				    let drag_state_snapshot = graph().drag_state.clone();

				    match &drag_state_snapshot {
				        crate::ui::graph_store::DragState::CreatingEdge {
				            source_id,
				            source_x,
				            source_y,
				            ..
				        } => {
				            // Check if moved significantly to convert to drag
				            // Convert to dragging state
				            graph
				                // Still pending click
				                .with_mut(|g| {
				                    // Update node position during drag

				                    g.drag_state = crate::ui::graph_store::DragState::CreatingEdge {
				                        source_id: source_id.clone(),
				                        source_x: *source_x,
				                        source_y: *source_y,
				                        mouse_x: coords.x,
				                        mouse_y: coords.y,
				                    };
				                });
				        }
				        crate::ui::graph_store::DragState::Lasso { start_x, start_y, .. } => {
				            graph
				                .with_mut(|g| {
				                    g.drag_state = crate::ui::graph_store::DragState::Lasso {
				                        start_x: *start_x,
				                        start_y: *start_y,
				                        current_x: coords.x,
				                        current_y: coords.y,
				                    };
				                });
				        }
				        crate::ui::graph_store::DragState::ClickPending {
				            node_id,
				            start_x,
				            start_y,
				            ..
				        } => {
				            let distance_moved = ((coords.x - start_x).powi(2)
				                + (coords.y - start_y).powi(2))
				                .sqrt();
				            if distance_moved > 5.0 {
				                graph
				                    .with_mut(|g| {
				                        g.drag_state = crate::ui::graph_store::DragState::Dragging {
				                            node_id: node_id.clone(),
				                            offset_x: coords.x - start_x,
				                            offset_y: coords.y - start_y,
				                        };
				                    });
				            } else {
				                graph
				                    .with_mut(|g| {
				                        g.drag_state = crate::ui::graph_store::DragState::ClickPending {
				                            node_id: node_id.clone(),
				                            start_x: *start_x,
				                            start_y: *start_y,
				                            mouse_x: coords.x,
				                            mouse_y: coords.y,
				                        };
				                    });
				            }
				        }
				        crate::ui::graph_store::DragState::Dragging {
				            node_id,
				            offset_x,
				            offset_y,
				        } => {
				            let new_x = coords.x - offset_x;
				            let new_y = coords.y - offset_y;
				            graph
				                .with_mut(|g| {
				                    g.set_position(node_id, new_x, new_y);
				                    g.drag_state = crate::ui::graph_store::DragState::Dragging {
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

				            crate::ui::graph_store::DragState::CreatingEdge { source_id: _, .. } => {}
				            crate::ui::graph_store::DragState::Lasso {
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
				                        g.drag_state = crate::ui::graph_store::DragState::None;
				                    });
				            }
				            crate::ui::graph_store::DragState::ClickPending {
				                node_id,
				                start_x,
				                start_y,
				                mouse_x,
				                mouse_y,
				            } => {
				                let distance_moved = ((mouse_x - start_x).powi(2)
				                    + (mouse_y - start_y).powi(2))
				                    .sqrt();
				                if distance_moved < 5.0 {
				                    let node_kind = graph()
				                        .find_node(&node_id)
				                        .map(|n| n.kind.clone());
				                    if let Some(kind) = node_kind {
				                        if kind.is_expandable() {
				                            graph
				                                .with_mut(|g| {
				                                    g.toggle_expand(&node_id);
				                                });
				                        }
				                    }
				                }
				                graph
				                    .with_mut(|g| {
				                        g.drag_state = crate::ui::graph_store::DragState::None;
				                    });
				            }
				            crate::ui::graph_store::DragState::Dragging { node_id, .. } => {
				                if let Some(node) = graph().find_node(&node_id) {
				                    let db_clone = db.clone();
				                    let node_id_clone = node_id.clone();
				                    let x = node.position.x;
				                    let y = node.position.y;
				                    spawn(async move {
				                        if let Err(e) = crate::ui::graph_store::save_node_position(
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
				                        g.drag_state = crate::ui::graph_store::DragState::None;
				                    });
				            }
				            _ => {
				                graph
				                    .with_mut(|g| {
				                        g.drag_state = crate::ui::graph_store::DragState::None;
				                    });
				            }
				        }
				    }
				},

				// SVG overlay for edges and interactions
				GraphSvgOverlay { graph, canvas_width, canvas_height }

				// Render visible nodes
				for node in graph().visible_nodes().iter() {
					GraphNodeComponent { graph, node: (*node).clone() }
				}
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
									            match crate::ui::graph_store::add_remote_machine(
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
