use std::collections::HashSet;

use dioxus::prelude::*;
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};
use tracing::{error, info, warn};

use crate::db::DbHandle;
use crate::ui::graph_types::*;

const CONTAINER_WIDTH: f64 = 200.0;
const CONTAINER_GAP: f64 = 32.0;
const NODE_HEIGHT: f64 = 36.0;
const NODE_GAP: f64 = 4.0;
const GRAPH_PADDING: f64 = 24.0;

// ─── Typed DB response structs ───────────────────────────────

#[derive(Debug, Clone, SurrealValue)]
struct MachineRow {
    id: RecordId,
    name: String,
}

#[derive(Debug, Clone, SurrealValue)]
struct DriveRow {
    id: RecordId,
    name: String,
    connected: bool,
    mount_point: Option<String>,
}

#[derive(Debug, Clone, SurrealValue)]
struct LocationRow {
    id: RecordId,
    machine: Option<RecordId>,
    drive: Option<RecordId>,
    path: String,
}

#[derive(Debug, Clone, SurrealValue)]
struct IntentRow {
    id: RecordId,
    source: RecordId,
    destinations: Vec<RecordId>,
    status: String,
    total_files: i64,
    completed_files: i64,
    created_at: String,
}

#[derive(Debug, Clone, SurrealValue)]
struct ReviewCountRow {
    count: i64,
}

// ─── Helpers ─────────────────────────────────────────────────

fn rid_string(id: &RecordId) -> String {
    let table = id.table.to_string();
    match &id.key {
        RecordIdKey::String(s) => format!("{table}:{s}"),
        RecordIdKey::Number(n) => format!("{table}:{n}"),
        _ => format!("{table}:{:?}", id.key),
    }
}

fn parse_rid(s: &str) -> Option<(&str, &str)> {
    s.split_once(':')
}

// ─── Interaction state ───────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum DragState {
    None,
    CreatingEdge {
        source_id: String,
        source_x: f64,
        source_y: f64,
        mouse_x: f64,
        mouse_y: f64,
    },
    Lasso {
        start_x: f64,
        start_y: f64,
        current_x: f64,
        current_y: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum AddPanelState {
    Closed,
    PickTarget,
    AddMachine,
}

// ─── Component ───────────────────────────────────────────────

#[component]
pub fn MappingGraph(refresh_tick: u32, on_changed: EventHandler) -> Element {
    let db = use_context::<DbHandle>();
    let mut drag = use_signal(|| DragState::None);
    let mut selected = use_signal(|| HashSet::<String>::new());
    let mut add_panel = use_signal(|| AddPanelState::Closed);

    // Add-machine form fields
    let mut machine_name = use_signal(|| String::new());
    let mut machine_host = use_signal(|| String::new());
    let mut machine_user = use_signal(|| String::new());

    let db_for_resource = db.clone();

    let graph_data = use_resource(move || {
        let db = db_for_resource.clone();
        let _tick = refresh_tick;
        async move { load_graph_data(&db).await }
    });

    let (containers, nodes, edges, review_count) = match &*graph_data.read() {
        Some(Ok(data)) => data.clone(),
        Some(Err(e)) => {
            error!("graph load failed: {}", e);
            (Vec::new(), Vec::new(), Vec::new(), 0i64)
        }
        None => (Vec::new(), Vec::new(), Vec::new(), 0i64),
    };

    let canvas_width = if containers.is_empty() {
        800.0
    } else {
        (containers.len() as f64) * (CONTAINER_WIDTH + CONTAINER_GAP) + CONTAINER_GAP + GRAPH_PADDING * 2.0
    };
    let canvas_height = 600.0;

    let node_positions: Vec<(String, f64, f64)> = nodes
        .iter()
        .map(|n| (rid_string(&n.id), n.center_x(), n.center_y()))
        .collect();

    // Status indicator text
    let status_class = if review_count > 0 { "status-indicator error" } else { "status-indicator ok" };
    let status_count = review_count;

    rsx! {
        div { class: "graph-area",
            // Top bar: status indicator + add button
            div { class: "graph-toolbar",
                // Status indicator
                div { class: "{status_class}",
                    if review_count > 0 {
                        span { class: "status-count", "{status_count}" }
                        span { class: "status-label",
                            if review_count == 1 { "issue" } else { "issues" }
                        }
                    }
                }
                // Add button
                button {
                    class: "btn-add",
                    onclick: move |_| *add_panel.write() = AddPanelState::PickTarget,
                    "+"
                }
            }

            div {
                class: "graph-wrapper",
                // Mouse handlers for drag/lasso
                onmousedown: {
                    let nodes_for_click = nodes.clone();
                    move |e: MouseEvent| {
                        // Shift+drag on empty space = lasso
                        if e.modifiers().shift() {
                            let coords = e.page_coordinates();
                            *drag.write() = DragState::Lasso {
                                start_x: coords.x,
                                start_y: coords.y,
                                current_x: coords.x,
                                current_y: coords.y,
                            };
                        } else {
                            // Click on empty space = deselect all
                            selected.write().clear();
                        }
                    }
                },
                onmousemove: {
                    move |e: MouseEvent| {
                        let current = drag.read().clone();
                        let coords = e.page_coordinates();
                        match current {
                            DragState::CreatingEdge { source_id, source_x, source_y, .. } => {
                                *drag.write() = DragState::CreatingEdge {
                                    source_id, source_x, source_y,
                                    mouse_x: coords.x, mouse_y: coords.y,
                                };
                            }
                            DragState::Lasso { start_x, start_y, .. } => {
                                *drag.write() = DragState::Lasso {
                                    start_x, start_y,
                                    current_x: coords.x, current_y: coords.y,
                                };
                            }
                            _ => {}
                        }
                    }
                },
                onmouseup: {
                    let nodes_for_lasso = nodes.clone();
                    move |_| {
                        let current = drag.read().clone();
                        match current {
                            DragState::CreatingEdge { .. } => {
                                info!("drag cancelled (released on empty space)");
                            }
                            DragState::Lasso { start_x, start_y, current_x, current_y } => {
                                // Select nodes within the lasso rectangle
                                let min_x = start_x.min(current_x);
                                let max_x = start_x.max(current_x);
                                let min_y = start_y.min(current_y);
                                let max_y = start_y.max(current_y);

                                let mut sel = selected.write();
                                for node in &nodes_for_lasso {
                                    let cx = node.center_x();
                                    let cy = node.center_y();
                                    if cx >= min_x && cx <= max_x && cy >= min_y && cy <= max_y {
                                        sel.insert(rid_string(&node.id));
                                    }
                                }
                            }
                            _ => {}
                        }
                        *drag.write() = DragState::None;
                    }
                },

                div {
                    class: "graph-html-layer",
                    style: "width: {canvas_width}px; min-height: {canvas_height}px;",

                    // SVG overlay for edges + rubber band + lasso
                    svg {
                        class: "graph-svg-overlay",
                        width: "{canvas_width}",
                        height: "{canvas_height}",
                        style: "width: {canvas_width}px; height: {canvas_height}px;",

                        // Edges
                        for edge in edges.iter() {
                            {
                                let source_pos = node_positions.iter().find(|(id, _, _)| *id == edge.source_id);
                                let dest_pos = node_positions.iter().find(|(id, _, _)| *id == edge.dest_id);
                                if let (Some((_, sx, sy)), Some((_, dx, dy))) = (source_pos, dest_pos) {
                                    let path_d = bezier_path(*sx, *sy, *dx, *dy);
                                    let color = edge_color(&edge.status);
                                    let width = if edge.status == "transferring" || edge.status == "scanning" { "3" } else { "2" };
                                    let key = rid_string(&edge.intent_id);
                                    rsx! {
                                        path {
                                            key: "{key}",
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

                        // Rubber-band line
                        if let DragState::CreatingEdge { source_x, source_y, mouse_x, mouse_y, .. } = *drag.read() {
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

                        // Lasso rectangle
                        if let DragState::Lasso { start_x, start_y, current_x, current_y } = *drag.read() {
                            {
                                let lx = start_x.min(current_x);
                                let ly = start_y.min(current_y);
                                let lw = (current_x - start_x).abs();
                                let lh = (current_y - start_y).abs();
                                rsx! {
                                    rect {
                                        x: "{lx}",
                                        y: "{ly}",
                                        width: "{lw}",
                                        height: "{lh}",
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

                    // HTML containers
                    for container in containers.iter() {
                        {
                            let cid = rid_string(&container.id);
                            let container_nodes: Vec<&NodeView> = nodes
                                .iter()
                                .filter(|n| n.container_id == cid)
                                .collect();

                            let disconnected_class = if container.connected { "" } else { " disconnected" };
                            let kind_label = if container.connected { container.kind.as_str() } else { "offline" };

                            rsx! {
                                div {
                                    key: "{cid}",
                                    class: "graph-container{disconnected_class}",
                                    style: "left: {container.x}px; top: {container.y}px;",

                                    div { class: "container-header",
                                        div {
                                            class: "container-dot",
                                            style: "background: {container.color};",
                                        }
                                        span { class: "container-name", "{container.name}" }
                                        span { class: "container-kind", "{kind_label}" }
                                    }

                                    div { class: "container-nodes",
                                        for node in container_nodes.iter() {
                                            {
                                                let node_id_str = rid_string(&node.id);
                                                let node_cx = node.center_x();
                                                let node_cy = node.center_y();
                                                let db = db.clone();
                                                let on_changed = on_changed;
                                                let is_selected = selected().contains(&node_id_str);
                                                let node_class = match (node.depth > 0, is_selected) {
                                                    (false, false) => "graph-node",
                                                    (true, false) => "graph-node nested",
                                                    (false, true) => "graph-node selected",
                                                    (true, true) => "graph-node nested selected",
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
                                                                    // Shift+click: toggle selection
                                                                    let mut sel = selected.write();
                                                                    if sel.contains(&node_id_str) {
                                                                        sel.remove(&node_id_str);
                                                                    } else {
                                                                        sel.insert(node_id_str.clone());
                                                                    }
                                                                } else {
                                                                    // Normal drag: create edge
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

                                                        span { class: "node-label", "{node.label}" }
                                                        div { class: "node-handle" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Add panel overlay
            if *add_panel.read() != AddPanelState::Closed {
                div {
                    class: "add-panel-overlay",
                    onclick: move |_| *add_panel.write() = AddPanelState::Closed,

                    div {
                        class: "add-panel",
                        onclick: move |e: MouseEvent| e.stop_propagation(),

                        match &*add_panel.read() {
                            AddPanelState::PickTarget => rsx! {
                                div { class: "add-panel-title", "Add location" }
                                div { class: "add-panel-list",
                                    for container in containers.iter() {
                                        {
                                            let cid = rid_string(&container.id);
                                            let color = container.color.clone();
                                            let name = container.name.clone();
                                            let connected = container.connected;
                                            let detail = if connected { container.kind.clone() } else { "offline".into() };
                                            let mount_point = container.mount_point.clone();
                                            let db = db.clone();

                                            rsx! {
                                                div {
                                                    key: "{cid}",
                                                    class: "add-panel-item",
                                                    onclick: {
                                                        let cid = cid.clone();
                                                        let mount_point = mount_point.clone();
                                                        let db = db.clone();
                                                        move |_| {
                                                            if !connected {
                                                                warn!("cannot add to disconnected target");
                                                                return;
                                                            }
                                                            let cid = cid.clone();
                                                            let mount_point = mount_point.clone();
                                                            let db = db.clone();
                                                            let on_changed = on_changed;
                                                            let mut add_panel = add_panel;
                                                            spawn(async move {
                                                                *add_panel.write() = AddPanelState::Closed;
                                                                match pick_and_add(&db, &cid, mount_point.as_deref()).await {
                                                                    Ok(true) => {
                                                                        info!("location added via picker");
                                                                        on_changed.call(());
                                                                    }
                                                                    Ok(false) => info!("picker cancelled"),
                                                                    Err(e) => error!("add location failed: {}", e),
                                                                }
                                                            });
                                                        }
                                                    },
                                                    div {
                                                        class: "item-dot",
                                                        style: "background: {color};",
                                                    }
                                                    span { class: "item-name", "{name}" }
                                                    span { class: "item-detail", "{detail}" }
                                                }
                                            }
                                        }
                                    }
                                    div { class: "add-panel-divider" }
                                    div {
                                        class: "add-panel-item add-machine",
                                        onclick: move |_| {
                                            *machine_name.write() = String::new();
                                            *machine_host.write() = String::new();
                                            *machine_user.write() = String::new();
                                            *add_panel.write() = AddPanelState::AddMachine;
                                        },
                                        span { class: "item-name", "+ Add remote machine" }
                                    }
                                }
                            },
                            AddPanelState::AddMachine => rsx! {
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
                                            onclick: move |_| *add_panel.write() = AddPanelState::PickTarget,
                                            "Back"
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
                                                        match add_remote_machine(&db, &name, &host, &user).await {
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
                            },
                            AddPanelState::Closed => rsx! {},
                        }
                    }
                }
            }
        }
    }
}

// ─── Data loading ───────────────────────────────────────────

type GraphData = (Vec<ContainerView>, Vec<NodeView>, Vec<EdgeView>, i64);

async fn load_graph_data(db: &DbHandle) -> Result<GraphData, String> {
    let containers = load_containers(db).await?;
    let nodes = load_nodes(db, &containers).await?;
    let edges = load_edges(db).await?;
    let review_count = load_review_count(db).await.unwrap_or(0);
    info!(
        "graph: {} containers, {} nodes, {} edges, {} reviews",
        containers.len(), nodes.len(), edges.len(), review_count
    );
    Ok((containers, nodes, edges, review_count))
}

async fn load_containers(db: &DbHandle) -> Result<Vec<ContainerView>, String> {
    let mut containers = Vec::new();

    let mut resp = db.db
        .query("SELECT id, name FROM machine")
        .await.map_err(|e| e.to_string())?;
    let machines: Vec<MachineRow> = resp.take(0).map_err(|e| e.to_string())?;

    for (i, m) in machines.iter().enumerate() {
        let is_local = rid_string(&m.id) == "machine:local";
        containers.push(ContainerView {
            id: m.id.clone(),
            name: m.name.clone(),
            kind: if is_local { "local".into() } else { "remote".into() },
            color: palette_color(i).to_string(),
            x: GRAPH_PADDING + (i as f64) * (CONTAINER_WIDTH + CONTAINER_GAP),
            y: GRAPH_PADDING,
            connected: true,
            mount_point: if is_local { dirs_home() } else { None },
        });
    }

    let mut resp = db.db
        .query("SELECT id, name, connected, mount_point FROM drive")
        .await.map_err(|e| e.to_string())?;
    let drives: Vec<DriveRow> = resp.take(0).map_err(|e| e.to_string())?;

    let offset = containers.len();
    for (i, d) in drives.iter().enumerate() {
        containers.push(ContainerView {
            id: d.id.clone(),
            name: d.name.clone(),
            kind: "drive".into(),
            color: palette_color(offset + i).to_string(),
            x: GRAPH_PADDING + ((offset + i) as f64) * (CONTAINER_WIDTH + CONTAINER_GAP),
            y: GRAPH_PADDING,
            connected: d.connected,
            mount_point: d.mount_point.clone(),
        });
    }

    Ok(containers)
}

fn dirs_home() -> Option<String> {
    std::env::var("HOME").ok()
}

async fn load_nodes(
    db: &DbHandle,
    containers: &[ContainerView],
) -> Result<Vec<NodeView>, String> {
    let mut resp = db.db
        .query("SELECT id, machine, drive, path FROM location ORDER BY path ASC")
        .await.map_err(|e| e.to_string())?;
    let rows: Vec<LocationRow> = resp.take(0).map_err(|e| e.to_string())?;

    let mut grouped: std::collections::HashMap<String, Vec<&LocationRow>> =
        std::collections::HashMap::new();

    for row in &rows {
        let owner_id = row.machine.as_ref().or(row.drive.as_ref());
        let owner_id = match owner_id {
            Some(id) => id,
            None => { warn!("location {} has no owner", rid_string(&row.id)); continue; }
        };
        let container = match containers.iter().find(|c| c.id == *owner_id) {
            Some(c) => c,
            None => { warn!("location {} orphaned", rid_string(&row.id)); continue; }
        };
        grouped.entry(rid_string(&container.id)).or_default().push(row);
    }

    let mut nodes = Vec::new();
    let mut container_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    const HEADER_H: f64 = 44.0;
    const PADDING_X: f64 = 8.0;
    const INDENT_PX: f64 = 12.0;

    for container in containers {
        let cid = rid_string(&container.id);
        let group = match grouped.get(&cid) {
            Some(g) => g,
            None => continue,
        };
        let all_paths: Vec<&str> = group.iter().map(|r| r.path.as_str()).collect();

        for row in group {
            let depth = compute_depth(&row.path, &all_paths);
            let count = container_counts.entry(cid.clone()).or_insert(0);
            let node_y = container.y + HEADER_H + (*count as f64) * (NODE_HEIGHT + NODE_GAP) + NODE_GAP;
            *count += 1;

            let indent = depth as f64 * INDENT_PX;
            nodes.push(NodeView {
                id: row.id.clone(),
                container_id: cid.clone(),
                path: row.path.clone(),
                label: short_path(&row.path),
                x: container.x + PADDING_X + indent,
                y: node_y,
                width: CONTAINER_WIDTH - PADDING_X * 2.0 - indent,
                height: NODE_HEIGHT,
                depth,
            });
        }
    }

    Ok(nodes)
}

async fn load_edges(db: &DbHandle) -> Result<Vec<EdgeView>, String> {
    let mut resp = db.db
        .query(
            "SELECT id, source, destinations, status, total_files, completed_files, created_at
             FROM intent ORDER BY created_at DESC",
        )
        .await.map_err(|e| e.to_string())?;
    let rows: Vec<IntentRow> = resp.take(0).map_err(|e| e.to_string())?;

    let mut edges = Vec::new();
    for row in &rows {
        let dest_id = match row.destinations.first() {
            Some(d) => rid_string(d),
            None => continue,
        };
        edges.push(EdgeView {
            intent_id: row.id.clone(),
            source_id: rid_string(&row.source),
            dest_id,
            status: row.status.clone(),
            total_files: row.total_files,
            completed_files: row.completed_files,
        });
    }
    Ok(edges)
}

async fn load_review_count(db: &DbHandle) -> Result<i64, String> {
    let mut resp = db.db
        .query("SELECT count() AS count FROM review_item WHERE resolution IS NONE GROUP ALL")
        .await.map_err(|e| e.to_string())?;
    let rows: Vec<ReviewCountRow> = resp.take(0).map_err(|e| e.to_string())?;
    Ok(rows.first().map(|r| r.count).unwrap_or(0))
}

// ─── Actions ────────────────────────────────────────────────

async fn pick_and_add(db: &DbHandle, container_id: &str, root: Option<&str>) -> Result<bool, String> {
    let mut dialog = rfd::AsyncFileDialog::new().set_title("Choose file or folder");

    if let Some(root_path) = root {
        dialog = dialog.set_directory(root_path);
    }

    let picked = dialog.pick_file().await;

    match picked {
        Some(handle) => {
            let path = handle.path().to_string_lossy().to_string();
            info!("picked: {}", path);
            add_location(db, container_id, &path).await?;
            Ok(true)
        }
        None => Ok(false),
    }
}

async fn add_location(db: &DbHandle, container_id: &str, path: &str) -> Result<(), String> {
    let (table, key) = parse_rid(container_id).ok_or("Invalid container ID")?;

    let query = format!(
        "LET $container = type::record('{table}', $key);
         CREATE location CONTENT {{
             {table}: $container,
             path: $path,
             available: true,
             created_at: time::now(),
         }}"
    );

    db.db
        .query(&query)
        .bind(("key", key.to_string()))
        .bind(("path", path.to_string()))
        .await.map_err(|e| e.to_string())?
        .check().map_err(|e| e.to_string())?;

    Ok(())
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

async fn add_remote_machine(db: &DbHandle, name: &str, hostname: &str, ssh_user: &str) -> Result<(), String> {
    let display_name = if name.is_empty() { hostname } else { name };

    db.db
        .query(
            "CREATE machine CONTENT {
                name: $name,
                kind: 'remote',
                hostname: $hostname,
                is_current: false,
                ssh_user: $ssh_user,
                last_seen: time::now(),
                online: false,
            }",
        )
        .bind(("name", display_name.to_string()))
        .bind(("hostname", hostname.to_string()))
        .bind(("ssh_user", if ssh_user.is_empty() { "root".to_string() } else { ssh_user.to_string() }))
        .await.map_err(|e| e.to_string())?
        .check().map_err(|e| e.to_string())?;

    Ok(())
}
