//! Headless rendering tests for graph UI components.
//!
//! These prove that UI behaviour can be asserted on without a display server,
//! a browser, or a windowing system — which is what makes them runnable inside
//! the Docker image (and in CI).
//!
//! The approach: build a `VirtualDom` around the component under test, let
//! Dioxus render it, then assert on the resulting HTML. This is the same
//! renderer Dioxus uses for SSR, so it exercises the real `rsx!` output —
//! attributes, class names, computed styles, conditional branches — rather
//! than a mock.
//!
//! What this can and cannot cover:
//!   * covered — markup structure, computed geometry in inline styles,
//!     conditional classes (selected/pinned/visible), text content, and how
//!     props flow into the tree.
//!   * NOT covered — pointer interaction, drag gestures, actual layout as the
//!     browser computes it, or anything requiring a real event loop. Those
//!     would need a live renderer; see `notes/the_design/UI_TESTING.md`.

#![allow(non_snake_case)] // Dioxus components are PascalCase by convention.

use daemon::graph_store::Graph;
use dioxus::prelude::*;
use frontend::ui::graph_nodes::{DirNode, FileNode};
use kip_core::graph_types::{FileType, GraphNode, NodeKind, Vec2};

/// A node with known geometry, so assertions can pin exact values.
fn test_node(id: &str, kind: NodeKind) -> GraphNode {
	GraphNode {
		id: id.to_string(),
		label: format!("label-for-{id}"),
		path: format!("/tmp/{id}"),
		kind,
		parent_id: None,
		color: "#ff0000".to_string(),
		position: Vec2::new(120.0, 340.0),
		velocity: Vec2::default(),
		pinned: false,
		visible: true,
		width: 200.0,
		height: 40.0,
		fx: None,
		fy: None,
	}
}

/// Render a component tree to HTML with no display, browser, or event loop.
fn render(app: fn() -> Element) -> String {
	let mut dom = VirtualDom::new(app);
	dom.rebuild_in_place();
	dioxus_ssr::render(&dom)
}

#[test]
fn file_node_renders_label_and_position() {
	fn App() -> Element {
		let graph = use_signal(Graph::new);
		let node = test_node("file-1", NodeKind::File { file_type: FileType::Document });
		rsx! { FileNode { graph, node } }
	}

	let html = render(App);

	assert!(html.contains("label-for-file-1"), "node label should appear in the markup:\n{html}");
	// Geometry is computed into the inline style, so a layout regression is visible here.
	assert!(html.contains("120"), "x position should reach the rendered style:\n{html}");
	assert!(html.contains("340"), "y position should reach the rendered style:\n{html}");
}

#[test]
fn selected_node_renders_differently_from_unselected() {
	fn Unselected() -> Element {
		let graph = use_signal(Graph::new);
		let node = test_node("file-1", NodeKind::File { file_type: FileType::Document });
		rsx! { FileNode { graph, node } }
	}

	fn Selected() -> Element {
		let graph = use_signal(|| {
			let mut g = Graph::new();
			g.selected.insert("file-1".to_string());
			g
		});
		let node = test_node("file-1", NodeKind::File { file_type: FileType::Document });
		rsx! { FileNode { graph, node } }
	}

	let unselected = render(Unselected);
	let selected = render(Selected);

	assert_ne!(
		unselected, selected,
		"selection must change the rendered output, otherwise it is invisible to the user"
	);
}

#[test]
fn directory_node_renders() {
	fn App() -> Element {
		let graph = use_signal(Graph::new);
		let node = test_node("dir-1", NodeKind::Directory { expanded: false });
		rsx! { DirNode { graph, node } }
	}

	let html = render(App);
	assert!(!html.is_empty(), "directory node should render something");
	assert!(html.contains("label-for-dir-1"), "directory label should appear:\n{html}");
}
