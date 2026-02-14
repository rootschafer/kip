use std::collections::HashSet;
use std::collections::HashMap;

use dioxus::prelude::*;
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};
use tracing::{error, info, warn};

use crate::db::DbHandle;
use crate::ui::file_picker::PickerManager;
use crate::ui::graph_types::*;
use crate::ui::container_components::*;

// Workspace grid layout constants (temporary until force-directed layout)
const GRID_COLS: usize = 4;
const GRID_SPACING_X: f64 = 180.0;
const GRID_SPACING_Y: f64 = 100.0;
const WORKSPACE_PADDING: f64 = 40.0;
const NODE_WIDTH_FILE: f64 = 150.0;
const NODE_HEIGHT_FILE: f64 = 36.0;
const NODE_SIZE_DIR: f64 = 56.0;

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

pub(crate) fn rid_string(id: &RecordId) -> String {
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
pub(crate) enum DragState {
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
    AddMachine,
}

// ─── Component ───────────────────────────────────────────────

#[component]
pub fn MappingGraph(refresh_tick: u32, on_changed: EventHandler) -> Element {
    let db = use_context::<DbHandle>();
    let mut picker = use_context::<PickerManager>();
    let mut drag = use_signal(|| DragState::None);
    let mut selected = use_signal(|| HashSet::<String>::new());
    let mut expansion_state = use_signal(|| HashMap::<String, (bool, bool)>::new());
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

    // Build color map for node tinting
    let color_map: HashMap<String, String> = containers.iter()
        .map(|c| (rid_string(&c.id), c.color.clone()))
        .collect();

    let canvas_width = 1200.0_f64;
    let canvas_height = 800.0_f64;

    let node_positions: Vec<(String, f64, f64)> = nodes
        .iter()
        .map(|n| (rid_string(&n.id), n.center_x(), n.center_y()))
        .collect();

    // Status indicator text
    let status_class = if review_count > 0 { "status-indicator error" } else { "status-indicator ok" };
    let status_count = review_count;

    // Pre-compute lasso rect (RSX macro can't handle let bindings inside if-let)
    let (lasso_active, lasso_x, lasso_y, lasso_w, lasso_h) = {
        let d = drag.read();
        match &*d {
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

    rsx! {
        div {
            class: "graph-area",
            // ─── Toolbar: status + machine chips + add button ───
            div {
                class: "graph-toolbar",
                div {
                    class: "{status_class}",
                    if review_count > 0 {
                        span { class: "status-count", "{status_count}" }
                        span {
                            class: "status-label",
                            if review_count == 1 { "issue" } else { "issues" }
                        }
                    }
                }
                div {
                    class: "machine-chips",
                    for container in containers.iter() {
                        MachineChip {
                            container: container.clone(),
                            on_click: move |c: ContainerView| {
                                if !c.connected {
                                    warn!("cannot add to disconnected target");
                                    return;
                                }
                                let cid = rid_string(&c.id);
                                let name = c.name.clone();
                                let root = c.mount_point.clone().unwrap_or_else(|| "/".to_string());
                                picker.open(cid, name, std::path::PathBuf::from(root));
                            },
                        }
                    }
                    button {
                        class: "btn-add",
                        onclick: move |_| {
                            *machine_name.write() = String::new();
                            *machine_host.write() = String::new();
                            *machine_user.write() = String::new();
                            *add_panel.write() = AddPanelState::AddMachine;
                        },
                        "+"
                    }
                }
            }

            // ─── Workspace: free nodes + SVG edges ───
            div {
                class: "workspace",
                onmousedown: move |e: MouseEvent| {
                    if e.modifiers().shift() {
                        let coords = e.page_coordinates();
                        *drag.write() = DragState::Lasso {
                            start_x: coords.x,
                            start_y: coords.y,
                            current_x: coords.x,
                            current_y: coords.y,
                        };
                    } else {
                        selected.write().clear();
                    }
                },
                onmousemove: move |e: MouseEvent| {
                    let current = drag.read().clone();
                    let coords = e.page_coordinates();
                    match current {
                        DragState::CreatingEdge { source_id, source_x, source_y, .. } => {
                            *drag.write() = DragState::CreatingEdge {
                                source_id,
                                source_x,
                                source_y,
                                mouse_x: coords.x,
                                mouse_y: coords.y,
                            };
                        }
                        DragState::Lasso { start_x, start_y, .. } => {
                            *drag.write() = DragState::Lasso {
                                start_x,
                                start_y,
                                current_x: coords.x,
                                current_y: coords.y,
                            };
                        }
                        _ => {}
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

                // SVG overlay for edges + rubber band + lasso
                svg {
                    class: "workspace-svg",
                    width: "{canvas_width}",
                    height: "{canvas_height}",
                    style: "width: {canvas_width}px; height: {canvas_height}px;",
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

                // Free nodes in workspace
                for node in nodes.iter() {
                    {
                        let color = color_map.get(&node.container_id).cloned().unwrap_or_default();
                        rsx! {
                            WorkspaceNode {
                                node: node.clone(),
                                color: color,
                                selected: selected,
                                drag: drag,
                                expansion_state: expansion_state,
                                db: db.clone(),
                                on_changed: on_changed,
                            }
                        }
                    }
                }
            }

            // ─── Add machine panel ───
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
            x: 0.0,  // Positioning is handled by toolbar layout
            y: 0.0,  // Positioning is handled by toolbar layout
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
            x: 0.0,  // Positioning is handled by toolbar layout
            y: 0.0,  // Positioning is handled by toolbar layout
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

    // Group locations by their owner (machine or drive)
    let mut grouped: HashMap<String, Vec<&LocationRow>> = HashMap::new();
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

    // Build nodes with workspace-absolute grid positions
    let mut nodes = Vec::new();
    let mut node_index = 0usize;

    for container in containers {
        let cid = rid_string(&container.id);
        
        // Add root folder node for each container that has a mount point
        if let Some(mount_point) = &container.mount_point {
            // Check if this mount point already exists as a location to avoid duplicates
            let already_exists = grouped.get(&cid).map_or(false, |group| {
                group.iter().any(|row| row.path == *mount_point)
            });
            
            if !already_exists {
                // Count direct children in the filesystem for this root directory
                let child_count = count_direct_children_in_filesystem(mount_point)?;
                
                let is_dir = true; // Root directories are always directories
                let depth = 0; // Root level
                
                // Grid position in workspace
                let col = node_index % GRID_COLS;
                let row_num = node_index / GRID_COLS;
                let (width, height) = if is_dir {
                    (NODE_SIZE_DIR, NODE_SIZE_DIR)
                } else {
                    (NODE_WIDTH_FILE, NODE_HEIGHT_FILE)
                };

                nodes.push(NodeView {
                    id: create_virtual_record_id(&format!("{}_root", cid)), // Create a virtual ID for the root
                    container_id: cid.clone(),
                    path: mount_point.clone(),
                    label: short_path(mount_point),
                    x: WORKSPACE_PADDING + (col as f64) * GRID_SPACING_X,
                    y: WORKSPACE_PADDING + (row_num as f64) * GRID_SPACING_Y,
                    width,
                    height,
                    depth,
                    is_dir,
                    is_expanded: false,
                    is_orbit: false,
                    child_count,
                });
                node_index += 1;
            }
        }
        
        // Process existing locations for this container
        let group = match grouped.get(&cid) {
            Some(g) => g,
            None => continue,
        };
        let all_paths: Vec<&str> = group.iter().map(|r| r.path.as_str()).collect();

        for row in group {
            let depth = compute_depth(&row.path, &all_paths);

            // Count direct children among sibling locations
            let parent_parts: Vec<&str> = row.path.split('/').filter(|s| !s.is_empty()).collect();
            let child_count = all_paths.iter()
                .filter(|&&other| {
                    if !path_contains(&row.path, other) { return false; }
                    let child_parts: Vec<&str> = other.split('/').filter(|s| !s.is_empty()).collect();
                    child_parts.len() == parent_parts.len() + 1
                })
                .count();

            let is_dir = if child_count > 0 {
                true
            } else {
                std::path::Path::new(&row.path).is_dir()
            };

            // Grid position in workspace
            let col = node_index % GRID_COLS;
            let row_num = node_index / GRID_COLS;
            let (width, height) = if is_dir {
                (NODE_SIZE_DIR, NODE_SIZE_DIR)
            } else {
                (NODE_WIDTH_FILE, NODE_HEIGHT_FILE)
            };

            nodes.push(NodeView {
                id: row.id.clone(),
                container_id: cid.clone(),
                path: row.path.clone(),
                label: short_path(&row.path),
                x: WORKSPACE_PADDING + (col as f64) * GRID_SPACING_X,
                y: WORKSPACE_PADDING + (row_num as f64) * GRID_SPACING_Y,
                width,
                height,
                depth,
                is_dir,
                is_expanded: false,
                is_orbit: false,
                child_count,
            });
            node_index += 1;
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

fn count_direct_children_in_filesystem(path: &str) -> Result<usize, String> {
    let path = std::path::Path::new(path);
    if !path.exists() {
        return Ok(0);
    }
    
    if !path.is_dir() {
        return Ok(0);
    }
    
    let mut count = 0;
    match std::fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    if entry.path().is_dir() || entry.path().is_file() {
                        count += 1;
                    }
                }
            }
        }
        Err(e) => {
            return Err(format!("Failed to read directory {}: {}", path.display(), e));
        }
    }
    
    Ok(count)
}

fn create_virtual_record_id(key: &str) -> RecordId {
    RecordId {
        table: "virtual".into(),
        key: surrealdb::types::RecordIdKey::String(key.to_string()),
    }
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
