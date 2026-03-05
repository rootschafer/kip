use dioxus::prelude::*;

use daemon::{DragState, Graph};
use kip_core::graph_types::*;

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

			// Render all visible edges - connect node centers with straight lines
			for edge in visible_edges.iter() {
				{
					let source_node = graph_snapshot.find_node(&edge.source_id);
					let dest_node = graph_snapshot.find_node(&edge.dest_id);
					if let (Some(source), Some(dest)) = (source_node, dest_node) {
						let sx = source.center_x();
						let sy = source.center_y();
						let dx = dest.center_x();
						let dy = dest.center_y();

						let path_d = bezier_path(sx, sy, dx, dy);
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
						opacity: "0.5",
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
