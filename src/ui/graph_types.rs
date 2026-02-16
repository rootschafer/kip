use std::ops::{Add, Sub, Mul, AddAssign, SubAssign};
use surrealdb::types::RecordId;

// ─── Vec2 ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self { Self { x, y } }
    pub fn length(&self) -> f64 { (self.x * self.x + self.y * self.y).sqrt() }
    pub fn normalized(&self) -> Self {
        let len = self.length();
        if len < 1e-10 { Self::default() } else { Self { x: self.x / len, y: self.y / len } }
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { Self { x: self.x + rhs.x, y: self.y + rhs.y } }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { Self { x: self.x - rhs.x, y: self.y - rhs.y } }
}

impl Mul<f64> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self { Self { x: self.x * rhs, y: self.y * rhs } }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) { self.x += rhs.x; self.y += rhs.y; }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) { self.x -= rhs.x; self.y -= rhs.y; }
}

// ─── Node types ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    File,
    Directory { expanded: bool },
    Group { expanded: bool },
    Machine,
    Drive { connected: bool },
}

impl NodeKind {
    pub fn is_expandable(&self) -> bool {
        matches!(self, NodeKind::Directory { .. } | NodeKind::Group { .. })
    }

    pub fn is_expanded(&self) -> bool {
        match self {
            NodeKind::Directory { expanded } | NodeKind::Group { expanded } => *expanded,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub path: String,
    pub kind: NodeKind,
    pub parent_id: Option<String>,
    pub color: String,
    pub position: Vec2,
    pub velocity: Vec2,
    pub pinned: bool,
    pub visible: bool,
    pub width: f64,
    pub height: f64,
}

impl GraphNode {
    pub fn center_x(&self) -> f64 { self.position.x + self.width / 2.0 }
    pub fn center_y(&self) -> f64 { self.position.y + self.height / 2.0 }
    pub fn center(&self) -> Vec2 { Vec2::new(self.center_x(), self.center_y()) }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphEdge {
    pub id: String,
    pub source_id: String,
    pub dest_id: String,
    pub status: String,
    pub total_files: i64,
    pub completed_files: i64,
    pub created_at: String,
}

// ─── Container (for toolbar chips) ────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerView {
    pub id: RecordId,
    pub name: String,
    pub kind: String,
    pub color: String,
    pub connected: bool,
    pub mount_point: Option<String>,
}

// ─── Visual helpers ───────────────────────────────────────────

pub const PALETTE: &[&str] = &[
    "#4a9eff", // blue
    "#3fb950", // green
    "#d29922", // orange
    "#bc8cff", // purple
    "#f78166", // coral
    "#58a6ff", // light blue
];

pub fn palette_color(index: usize) -> &'static str {
    PALETTE[index % PALETTE.len()]
}

pub fn bezier_path(x1: f64, y1: f64, x2: f64, y2: f64) -> String {
    let dx = (x2 - x1).abs() * 0.5;
    format!(
        "M {x1} {y1} C {} {y1}, {} {y2}, {x2} {y2}",
        x1 + dx,
        x2 - dx
    )
}

pub fn edge_color(status: &str) -> &'static str {
    match status {
        "idle" => "#555",
        "scanning" | "transferring" => "#4a9eff",
        "complete" => "#3fb950",
        "needs_review" => "#d29922",
        "failed" => "#f85149",
        _ => "#555",
    }
}

pub fn short_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() <= 2 {
        path.to_string()
    } else {
        format!(".../{}", parts[parts.len() - 2..].join("/"))
    }
}

pub fn path_contains(parent: &str, child: &str) -> bool {
    if parent == child { return false; }
    let parent_normalized = if parent.ends_with('/') {
        parent.to_string()
    } else {
        format!("{parent}/")
    };
    child.starts_with(&parent_normalized)
}

pub fn is_direct_child(parent_path: &str, child_path: &str) -> bool {
    if !path_contains(parent_path, child_path) { return false; }
    let parent_clean = parent_path.trim_end_matches('/');
    let child_clean = child_path.trim_end_matches('/');
    let remaining = &child_clean[parent_clean.len()..];
    let remaining_trimmed = remaining.trim_start_matches('/');
    let parts: Vec<&str> = remaining_trimmed.split('/').filter(|s| !s.is_empty()).collect();
    parts.len() == 1
}

// ─── Node sizing ──────────────────────────────────────────────

const NODE_WIDTH_FILE: f64 = 150.0;
const NODE_HEIGHT_FILE: f64 = 36.0;
const DIR_MIN_SIZE: f64 = 50.0;
const DIR_MAX_SIZE: f64 = 90.0;
const MACHINE_SIZE: f64 = 70.0;

pub fn node_dimensions(kind: &NodeKind, child_count: usize) -> (f64, f64) {
    match kind {
        NodeKind::File => (NODE_WIDTH_FILE, NODE_HEIGHT_FILE),
        NodeKind::Directory { .. } | NodeKind::Group { .. } => {
            let size = (DIR_MIN_SIZE + (1.0 + child_count as f64).ln() * 10.0)
                .clamp(DIR_MIN_SIZE, DIR_MAX_SIZE);
            (size, size)
        }
        NodeKind::Machine | NodeKind::Drive { .. } => (MACHINE_SIZE, MACHINE_SIZE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_contains() {
        assert!(path_contains("/a/b", "/a/b/c"));
        assert!(!path_contains("/a/b", "/a/bc"));
        assert!(!path_contains("/a/b", "/a/b"));
    }

    #[test]
    fn test_is_direct_child() {
        assert!(is_direct_child("/a/b", "/a/b/c"));
        assert!(!is_direct_child("/a/b", "/a/b/c/d"));
        assert!(!is_direct_child("/a/b", "/a/b"));
    }

    #[test]
    fn test_vec2_ops() {
        let a = Vec2::new(1.0, 2.0);
        let b = Vec2::new(3.0, 4.0);
        let c = a + b;
        assert_eq!(c.x, 4.0);
        assert_eq!(c.y, 6.0);
    }
}
