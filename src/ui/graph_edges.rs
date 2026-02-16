use dioxus::prelude::*;

use crate::ui::graph_types::*;
use crate::ui::graph_store::{Graph, DragState};

// ─── GraphSvgOverlay ───────────────────────────────────────────
// SVG overlay for rendering edges, rubber band, and lasso

#[component]
pub fn GraphSvgOverlay(
    graph: Signal<Graph>,
    canvas_width: f64,
    canvas_height: f64,
) -> Element {
    let graph_snapshot = graph();
    let visible_edges = graph_snapshot.visible_edges();
    let node_positions: Vec<(String, f64, f64)> = graph_snapshot
        .visible_nodes()
        .iter()
        .map(|node| (node.id.clone(), node.center_x(), node.center_y()))
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

    rsx! {
        svg {
            class: "workspace-svg",
            width: "{canvas_width}",
            height: "{canvas_height}",
            style: "width: {canvas_width}px; height: {canvas_height}px;",
            
            // Render all visible edges
            for edge in visible_edges.iter() {
                {
                    let source_pos = node_positions.iter().find(|(id, _, _)| *id == edge.source_id);
                    let dest_pos = node_positions.iter().find(|(id, _, _)| *id == edge.dest_id);
                    if let (Some((_, sx, sy)), Some((_, dx, dy))) = (source_pos, dest_pos) {
                        let path_d = bezier_path(*sx, *sy, *dx, *dy);
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
            if let DragState::CreatingEdge { source_x, source_y, mouse_x, mouse_y, .. } = drag_state_snapshot {
                line {
                    x1: "{source_x}",
                    y1: "{source_y}",
                    x2: "{mouse_x}",
                    y2: "{mouse_y}",
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