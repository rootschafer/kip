use dioxus::prelude::*;

use crate::ui::{
	graph_store::{DragState, Graph},
	graph_types::*,
};

// ─── Helper: Calculate edge attachment point on node bounds ────
/// Returns the point where an edge should connect to a node's boundary
/// instead of its center, preventing ugly loops
fn get_edge_attachment_point(
	node: &GraphNode,
	toward_x: f64,
	toward_y: f64,
) -> (f64, f64) {
	let center_x = node.center_x();
	let center_y = node.center_y();
	let half_w = node.width / 2.0;
	let half_h = node.height / 2.0;
	
	// Calculate angle from node center to target
	let dx = toward_x - center_x;
	let dy = toward_y - center_y;
	
	// For circular nodes (directories, machines, drives), use radius
	if matches!(node.kind, NodeKind::Directory { .. } | NodeKind::Group { .. } | NodeKind::Machine { .. } | NodeKind::Drive { .. }) {
		let radius = half_w; // Circular nodes have equal width/height
		if dx.abs() < 0.1 && dy.abs() < 0.1 {
			return (center_x, center_y);
		}
		let angle = dy.atan2(dx);
		let attach_x = center_x + radius * angle.cos();
		let attach_y = center_y + radius * angle.sin();
		return (attach_x, attach_y);
	}
	
	// For rectangular file nodes, calculate intersection with rectangle boundary
	if dx.abs() < 0.1 && dy.abs() < 0.1 {
		return (center_x, center_y);
	}
	
	// Calculate which side of the rectangle the edge should connect to
	let slope = dy / dx;
	let half_slope = half_h / half_w;
	
	let (attach_x, attach_y) = if slope.abs() <= half_slope {
		// Connect to left or right side
		if dx > 0.0 {
			(center_x + half_w, center_y + half_w * slope)
		} else {
			(center_x - half_w, center_y - half_w * slope)
		}
	} else {
		// Connect to top or bottom
		if dy > 0.0 {
			(center_x + half_h / slope, center_y + half_h)
		} else {
			(center_x - half_h / slope, center_y - half_h)
		}
	};
	
	(attach_x, attach_y)
}

// ─── GraphSvgOverlay ───────────────────────────────────────────
// SVG overlay for rendering edges, cluster backgrounds, rubber band, and lasso

#[component]
pub fn GraphSvgOverlay(
	graph: Signal<Graph>,
	canvas_width: f64,
	canvas_height: f64,
	viewport_scale: f64,
	viewport_x: f64,
	viewport_y: f64,
) -> Element {
	let graph_snapshot = graph();
	let visible_edges = graph_snapshot.visible_edges();
	
	// Store full node data for edge attachment calculation
	let node_data: std::collections::HashMap<String, (f64, f64, f64, f64, NodeKind)> = graph_snapshot
		.visible_nodes()
		.iter()
		.map(|node| {
			(
				node.id.clone(),
				(node.center_x(), node.center_y(), node.width, node.height, node.kind.clone()),
			)
		})
		.collect();

	// Pre-compute lasso rect
	let (lasso_active, lasso_x, lasso_y, lasso_w, lasso_h) = {
		let drag_state = &graph_snapshot.drag_state;
		match drag_state {
			DragState::Lasso { start_x, start_y, current_x, current_y } => (
				true,
				start_x.min(*current_x),
				start_y.min(*current_y),
				(current_x - start_x).abs(),
				(current_y - start_y).abs(),
			),
			_ => (false, 0.0, 0.0, 0.0, 0.0),
		}
	};

	// Capture drag state for the rubber band
	let drag_state_snapshot = &graph_snapshot.drag_state;

	// Pre-compute rubber-band line coordinates (transform mouse coords to graph space)
	let rubber_band_line =
		if let DragState::CreatingEdge { source_x, source_y, mouse_x, mouse_y, .. } = &drag_state_snapshot {
			let graph_mouse_x = (mouse_x - viewport_x) / viewport_scale;
			let graph_mouse_y = (mouse_y - viewport_y) / viewport_scale;
			Some((source_x, source_y, graph_mouse_x, graph_mouse_y))
		} else {
			None
		};

	rsx! {
		svg {
			class: "workspace-svg",
			width: "{canvas_width}",
			height: "{canvas_height}",
			style: "width: {canvas_width}px; height: {canvas_height}px;",

			// Cluster backgrounds removed

			// Render all visible edges with proper attachment points
			for edge in visible_edges.iter() {
				{
					let source = node_data.get(&edge.source_id);
					let dest = node_data.get(&edge.dest_id);
					if let (Some((sx, sy, sw, sh, skind)), Some((dx, dy, dw, dh, dkind))) = (source, dest) {
						// Create temporary node structs for attachment calculation
						let source_node = GraphNode {
							id: edge.source_id.clone(),
							label: String::new(),
							path: String::new(),
							kind: skind.clone(),
							parent_id: None,
							color: String::new(),
							position: crate::ui::graph_types::Vec2::new(*sx, *sy),
							velocity: crate::ui::graph_types::Vec2::default(),
							pinned: false,
							visible: true,
							width: *sw,
							height: *sh,
							fx: None,
							fy: None,
						};
						let dest_node = GraphNode {
							id: edge.dest_id.clone(),
							label: String::new(),
							path: String::new(),
							kind: dkind.clone(),
							parent_id: None,
							color: String::new(),
							position: crate::ui::graph_types::Vec2::new(*dx, *dy),
							velocity: crate::ui::graph_types::Vec2::default(),
							pinned: false,
							visible: true,
							width: *dw,
							height: *dh,
							fx: None,
							fy: None,
						};
						
						// Calculate attachment points on node boundaries
						let (start_x, start_y) = get_edge_attachment_point(&source_node, *dx, *dy);
						let (end_x, end_y) = get_edge_attachment_point(&dest_node, *sx, *sy);
						
						let path_d = bezier_path(start_x, start_y, end_x, end_y);
						let color = edge_color(&edge.status);
						let width = if edge.status == "transferring" || edge.status == "scanning" {
							"3"
						} else {
							"2"
						};

						rsx! {
					path {
						key: "{edge.id}",
						d: "{path_d}",
						stroke: "{color}",
						stroke_width: "{width}",
						fill: "none",
						stroke_linecap: "round",
						opacity: "0.7",
					}
				}
					} else {
						rsx! {}
					}
				}
			}

			// Rubber-band line during edge creation
			if let Some((sx, sy, mx, my)) = rubber_band_line {
				line {
					x1: "{sx}",
					y1: "{sy}",
					x2: "{mx}",
					y2: "{my}",
					stroke: "#4a9eff",
					stroke_width: "2",
					stroke_dasharray: "6 4",
					stroke_linecap: "round",
					opacity: "0.8",
				}
			}

			// Lasso rectangle during selection
			if lasso_active {
				rect {
					x: "{lasso_x}",
					y: "{lasso_y}",
					width: "{lasso_w}",
					height: "{lasso_h}",
					fill: "rgba(74, 158, 255, 0.08)",
					stroke: "#4a9eff",
					stroke_width: "1",
					stroke_dasharray: "4 3",
					rx: "4",
				}
			}
		}
	}
}
