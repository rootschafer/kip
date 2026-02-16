use std::collections::HashSet;

use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};
use tracing::{info, warn};

use crate::db::DbHandle;
use crate::ui::graph_types::*;

// ─── Force simulation constants ───────────────────────────────

const REPULSION: f64 = 500.0;
const SPRING_K: f64 = 0.05;
const SPRING_REST: f64 = 120.0;
const PARENT_K: f64 = 0.08;
const PARENT_REST: f64 = 80.0;
const CENTER_GRAVITY: f64 = 0.01;
const DAMPING: f64 = 0.9;
const ALPHA_DECAY: f64 = 0.995;
const ALPHA_MIN: f64 = 0.001;
const WARM_RESTART: f64 = 0.3;

// ─── Interaction state ────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum DragState {
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
    ClickPending {
        node_id: String,
        start_x: f64,
        start_y: f64,
        mouse_x: f64,
        mouse_y: f64,
    },
    Dragging {
        node_id: String,
        offset_x: f64,
        offset_y: f64,
    },
}

// ─── Graph state ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub alpha: f64,
    pub sim_running: bool,
    pub selected: HashSet<String>,
    pub drag_state: DragState,
    pub containers: Vec<ContainerView>,
    pub review_count: i64,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            alpha: 0.0,
            sim_running: false,
            selected: HashSet::new(),
            drag_state: DragState::None,
            containers: Vec::new(),
            review_count: 0,
        }
    }

    // ── Node queries ──

    pub fn find_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn find_node_mut(&mut self, id: &str) -> Option<&mut GraphNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn visible_nodes(&self) -> Vec<&GraphNode> {
        self.nodes.iter().filter(|n| n.visible).collect()
    }

    pub fn visible_edges(&self) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| {
            self.nodes.iter().any(|n| n.id == e.source_id && n.visible)
                && self.nodes.iter().any(|n| n.id == e.dest_id && n.visible)
        }).collect()
    }

    // ── Node mutations ──

    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.push(node);
        self.wake(WARM_RESTART);
    }

    pub fn remove_node(&mut self, id: &str) {
        self.nodes.retain(|n| n.id != id);
        self.edges.retain(|e| e.source_id != id && e.dest_id != id);
        self.wake(WARM_RESTART);
    }

    pub fn set_visible(&mut self, id: &str, visible: bool) {
        if let Some(node) = self.find_node_mut(id) {
            node.visible = visible;
            self.wake(WARM_RESTART);
        }
    }

    pub fn toggle_expand(&mut self, id: &str) {
        // Find the node's path and current expansion state
        let (path, was_expanded) = match self.find_node(id) {
            Some(n) => (n.path.clone(), n.kind.is_expanded()),
            None => return,
        };
        let new_expanded = !was_expanded;

        // Update the node's kind
        if let Some(node) = self.find_node_mut(id) {
            match &mut node.kind {
                NodeKind::Directory { expanded } | NodeKind::Group { expanded } => {
                    *expanded = new_expanded;
                }
                _ => return,
            }
        }

        // Collect child IDs to toggle visibility
        let child_ids: Vec<String> = self.nodes.iter()
            .filter(|n| is_direct_child(&path, &n.path))
            .map(|n| n.id.clone())
            .collect();

        // Toggle visibility of direct children
        for child_id in child_ids {
            if let Some(child) = self.find_node_mut(&child_id) {
                child.visible = new_expanded;
                // If collapsing, also collapse any expanded children recursively
                if !new_expanded {
                    match &mut child.kind {
                        NodeKind::Directory { expanded } | NodeKind::Group { expanded } => {
                            *expanded = false;
                        }
                        _ => {}
                    }
                }
            }
        }

        // If collapsing, hide all descendants (not just direct children)
        if !new_expanded {
            let descendant_ids: Vec<String> = self.nodes.iter()
                .filter(|n| path_contains(&path, &n.path))
                .map(|n| n.id.clone())
                .collect();
            for desc_id in descendant_ids {
                if let Some(desc) = self.find_node_mut(&desc_id) {
                    desc.visible = false;
                }
            }
        }

        self.wake(WARM_RESTART);
    }

    pub fn set_position(&mut self, id: &str, x: f64, y: f64) {
        if let Some(node) = self.find_node_mut(id) {
            node.position = Vec2::new(x, y);
            node.velocity = Vec2::default();
            node.pinned = true;
        }
    }

    // ── Edge mutations ──

    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
        self.wake(WARM_RESTART);
    }

    pub fn remove_edge(&mut self, id: &str) {
        self.edges.retain(|e| e.id != id);
    }

    // ── Selection ──

    pub fn toggle_select(&mut self, id: &str) {
        if self.selected.contains(id) {
            self.selected.remove(id);
        } else {
            self.selected.insert(id.to_string());
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    pub fn select_in_rect(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        for node in &self.nodes {
            if !node.visible { continue; }
            let cx = node.center_x();
            let cy = node.center_y();
            if cx >= min_x && cx <= max_x && cy >= min_y && cy <= max_y {
                self.selected.insert(node.id.clone());
            }
        }
    }

    // ── Simulation ──

    fn wake(&mut self, alpha: f64) {
        // Only wake if not already running, to prevent constant restarts
        if !self.sim_running {
            self.alpha = alpha;
            self.sim_running = true;
        } else {
            // If already running, just boost alpha slightly to maintain energy
            self.alpha = self.alpha.max(alpha * 0.5); // Lower boost when already running
        }
    }

    pub fn tick(&mut self) -> bool {
        // If simulation is not running, don't tick
        if !self.sim_running {
            return false;
        }

        apply_forces(&mut self.nodes, &self.edges, self.alpha);
        self.alpha *= ALPHA_DECAY;

        // Check if alpha has dropped below threshold - if so, stop simulation
        if self.alpha < ALPHA_MIN {
            self.sim_running = false;
            self.alpha = 0.0; // Ensure alpha is properly set to 0
            return false;
        }

        true  // Continue simulation if alpha is still above threshold
    }

    // ── Bulk load ──

    pub fn load_from_db(
        &mut self,
        containers: Vec<ContainerView>,
        nodes: Vec<GraphNode>,
        edges: Vec<GraphEdge>,
        review_count: i64,
    ) {
        self.containers = containers;
        self.nodes = nodes;
        self.edges = edges;
        self.review_count = review_count;
        self.alpha = 1.0;
        self.sim_running = false; // Don't start simulation automatically
    }
}

// ─── Force-directed algorithm ─────────────────────────────────

fn apply_forces(nodes: &mut [GraphNode], edges: &[GraphEdge], alpha: f64) {
    let n = nodes.len();

    // Collect visible indices for O(1) lookup
    let visible: Vec<usize> = (0..n).filter(|&i| nodes[i].visible).collect();

    // Workspace center (approximate)
    let center = Vec2::new(600.0, 400.0);

    // 1. Repulsion between all visible pairs
    for i in 0..visible.len() {
        for j in (i + 1)..visible.len() {
            let ai = visible[i];
            let bi = visible[j];

            let delta = nodes[bi].center() - nodes[ai].center();
            let dist = delta.length().max(1.0);
            let force_mag = REPULSION / (dist * dist);
            let force = delta.normalized() * force_mag * alpha;

            if !nodes[ai].pinned {
                nodes[ai].velocity -= force;
            }
            if !nodes[bi].pinned {
                nodes[bi].velocity += force;
            }
        }
    }

    // 2. Edge springs
    for edge in edges {
        let src_idx = nodes.iter().position(|n| n.id == edge.source_id);
        let dst_idx = nodes.iter().position(|n| n.id == edge.dest_id);
        if let (Some(si), Some(di)) = (src_idx, dst_idx) {
            if !nodes[si].visible || !nodes[di].visible { continue; }

            let delta = nodes[di].center() - nodes[si].center();
            let dist = delta.length().max(1.0);
            let displacement = dist - SPRING_REST;
            let force = delta.normalized() * SPRING_K * displacement * alpha;

            if !nodes[si].pinned {
                nodes[si].velocity += force;
            }
            if !nodes[di].pinned {
                nodes[di].velocity -= force;
            }
        }
    }

    // 3. Parent-child springs (location → machine/drive clustering)
    //    Also works for directory → child clustering
    let parent_pairs: Vec<(usize, usize)> = visible.iter()
        .filter_map(|&i| {
            nodes[i].parent_id.as_ref().and_then(|pid| {
                nodes.iter().position(|n| n.id == *pid && n.visible)
                    .map(|pi| (i, pi))
            })
        })
        .collect();

    for (child_idx, parent_idx) in parent_pairs {
        let delta = nodes[parent_idx].center() - nodes[child_idx].center();
        let dist = delta.length().max(1.0);
        let displacement = dist - PARENT_REST;
        let force = delta.normalized() * PARENT_K * displacement * alpha;

        if !nodes[child_idx].pinned {
            nodes[child_idx].velocity += force;
        }
        if !nodes[parent_idx].pinned {
            nodes[parent_idx].velocity -= force * 0.3; // Parents resist movement more
        }
    }

    // 4. Center gravity
    for &i in &visible {
        if nodes[i].pinned { continue; }
        let to_center = center - nodes[i].center();
        nodes[i].velocity += to_center * CENTER_GRAVITY * alpha;
    }

    // 5. Apply velocities with damping
    for &i in &visible {
        if nodes[i].pinned { continue; }
        nodes[i].velocity = nodes[i].velocity * DAMPING;
        nodes[i].position += nodes[i].velocity;

        // Clamp to workspace bounds
        nodes[i].position.x = nodes[i].position.x.clamp(20.0, 1160.0);
        nodes[i].position.y = nodes[i].position.y.clamp(20.0, 760.0);
    }
}

// ─── DB loading ───────────────────────────────────────────────

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
    graph_x: Option<f64>,
    graph_y: Option<f64>,
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

pub fn rid_string(id: &RecordId) -> String {
    let table = id.table.to_string();
    match &id.key {
        RecordIdKey::String(s) => format!("{table}:{s}"),
        RecordIdKey::Number(n) => format!("{table}:{n}"),
        _ => format!("{table}:{:?}", id.key),
    }
}

fn dirs_home() -> Option<String> {
    std::env::var("HOME").ok()
}

pub async fn load_graph_data(db: &DbHandle) -> Result<(Vec<ContainerView>, Vec<GraphNode>, Vec<GraphEdge>, i64), String> {
    tracing::info!("Starting to load graph data from database...");
    let containers = load_containers(db).await?;
    tracing::info!("Loaded {} containers", containers.len());
    
    let nodes = load_nodes(db, &containers).await?;
    tracing::info!("Loaded {} nodes", nodes.len());
    
    let edges = load_edges(db).await?;
    tracing::info!("Loaded {} edges", edges.len());
    
    let review_count = load_review_count(db).await.unwrap_or(0);
    tracing::info!("Loaded {} review items", review_count);

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
    
    tracing::info!("Found {} machines in database", machines.len());
    for (i, m) in machines.iter().enumerate() {
        let is_local = rid_string(&m.id) == "machine:local";
        tracing::info!("Loading machine: {} (id: {}, local: {})", m.name, rid_string(&m.id), is_local);
        containers.push(ContainerView {
            id: m.id.clone(),
            name: m.name.clone(),
            kind: if is_local { "local".into() } else { "remote".into() },
            color: palette_color(i).to_string(),
            connected: true,
            mount_point: if is_local { dirs_home() } else { None },
        });
    }

    let mut resp = db.db
        .query("SELECT id, name, connected, mount_point FROM drive")
        .await.map_err(|e| e.to_string())?;
    let drives: Vec<DriveRow> = resp.take(0).map_err(|e| e.to_string())?;
    
    tracing::info!("Found {} drives in database", drives.len());
    let offset = containers.len();
    for (i, d) in drives.iter().enumerate() {
        tracing::info!("Loading drive: {} (id: {}, connected: {})", d.name, rid_string(&d.id), d.connected);
        containers.push(ContainerView {
            id: d.id.clone(),
            name: d.name.clone(),
            kind: "drive".into(),
            color: palette_color(offset + i).to_string(),
            connected: d.connected,
            mount_point: d.mount_point.clone(),
        });
    }

    Ok(containers)
}

async fn load_nodes(
    db: &DbHandle,
    containers: &[ContainerView],
) -> Result<Vec<GraphNode>, String> {
    let mut nodes = Vec::new();

    // Create nodes for machines and drives
    for container in containers {
        let cid = rid_string(&container.id);
        let (w, h) = match container.kind.as_str() {
            "drive" => node_dimensions(&NodeKind::Drive { connected: container.connected }, 0),
            _ => node_dimensions(&NodeKind::Machine, 0),
        };

        nodes.push(GraphNode {
            id: cid.clone(),
            label: container.name.clone(),
            path: String::new(),
            kind: match container.kind.as_str() {
                "drive" => NodeKind::Drive { connected: container.connected },
                _ => NodeKind::Machine,
            },
            parent_id: None,
            color: container.color.clone(),
            position: random_start_position(),
            velocity: Vec2::default(),
            pinned: false,
            visible: true,
            width: w,
            height: h,
        });
    }

    // Load locations from DB
    tracing::info!("Loading locations from database...");
    let mut resp = db.db
        .query("SELECT id, machine, drive, path, graph_x, graph_y FROM location ORDER BY path ASC")
        .await.map_err(|e| e.to_string())?;
    let rows: Vec<LocationRow> = resp.take(0).map_err(|e| e.to_string())?;
    tracing::info!("Loaded {} locations from database", rows.len());

    // Build all location paths for child counting
    let all_paths: Vec<&str> = rows.iter().map(|r| r.path.as_str()).collect();

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

        let parent_rid = rid_string(owner_id);

        // Determine if directory
        let is_dir = std::path::Path::new(&row.path).is_dir()
            || all_paths.iter().any(|&other| path_contains(&row.path, other));

        // Count direct children among known locations
        let child_count = all_paths.iter()
            .filter(|&&other| is_direct_child(&row.path, other))
            .count();

        let kind = if is_dir {
            NodeKind::Directory { expanded: false }
        } else {
            NodeKind::File
        };

        let (w, h) = node_dimensions(&kind, child_count);

        // Use saved position if available
        let (position, pinned) = match (row.graph_x, row.graph_y) {
            (Some(x), Some(y)) => (Vec2::new(x, y), true),
            _ => (random_start_position(), false),
        };

        // Determine parent_id: find closest ancestor among existing locations,
        // falling back to the machine/drive owner
        let parent_id = rows.iter()
            .filter(|other| other.id != row.id && path_contains(&other.path, &row.path))
            .max_by_key(|other| other.path.len()) // closest ancestor = longest matching path
            .map(|other| rid_string(&other.id))
            .unwrap_or(parent_rid);

        // Top-level locations (direct children of machine/drive) are visible
        // Deeper locations start hidden
        let is_top_level = !rows.iter().any(|other| {
            other.id != row.id && path_contains(&other.path, &row.path)
        });

        nodes.push(GraphNode {
            id: rid_string(&row.id),
            label: short_path(&row.path),
            path: row.path.clone(),
            kind,
            parent_id: Some(parent_id),
            color: container.color.clone(),
            position,
            velocity: Vec2::default(),
            pinned,
            visible: is_top_level,
            width: w,
            height: h,
        });
    }

    Ok(nodes)
}

async fn load_edges(db: &DbHandle) -> Result<Vec<GraphEdge>, String> {
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
        edges.push(GraphEdge {
            id: rid_string(&row.id),
            source_id: rid_string(&row.source),
            dest_id,
            status: row.status.clone(),
            total_files: row.total_files,
            completed_files: row.completed_files,
            created_at: row.created_at.clone(),
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

// ─── DB actions ───────────────────────────────────────────────

pub async fn create_edge_in_db(db: &DbHandle, source_id: &str, dest_id: &str) -> Result<String, String> {
    let (_, src_key) = source_id.split_once(':').ok_or("Invalid source ID")?;
    let (_, dst_key) = dest_id.split_once(':').ok_or("Invalid dest ID")?;

    let mut resp = db.db
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
        .await.map_err(|e| e.to_string())?;

    resp.check().map_err(|e| e.to_string())?;
    Ok("created".into())
}

pub async fn add_remote_machine(db: &DbHandle, name: &str, hostname: &str, ssh_user: &str) -> Result<(), String> {
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

pub async fn save_node_position(db: &DbHandle, node_id: &str, x: f64, y: f64) -> Result<(), String> {
    // Only save positions for location nodes
    if !node_id.starts_with("location:") { return Ok(()); }

    let (_, key) = node_id.split_once(':').ok_or("Invalid node ID")?;

    db.db
        .query("UPDATE type::record('location', $key) SET graph_x = $x, graph_y = $y")
        .bind(("key", key.to_string()))
        .bind(("x", x))
        .bind(("y", y))
        .await.map_err(|e| e.to_string())?
        .check().map_err(|e| e.to_string())?;

    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────

fn random_start_position() -> Vec2 {
    // Spread initial positions around center to avoid all-at-origin
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::time::Instant::now().hash(&mut hasher);
    let h = hasher.finish();
    let x = 300.0 + ((h % 600) as f64);
    let y = 200.0 + (((h >> 16) % 400) as f64);
    Vec2::new(x, y)
}
