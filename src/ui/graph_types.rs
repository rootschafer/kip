use surrealdb::types::RecordId;

/// Color palette for machines/drives — designed for dark backgrounds.
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

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerView {
    pub id: RecordId,
    pub name: String,
    pub kind: String, // "local", "remote", or "drive"
    pub color: String,
    pub x: f64,
    pub y: f64,
    pub connected: bool,
    pub mount_point: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeView {
    pub id: RecordId,
    pub container_id: String,
    pub path: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Nesting depth: 0 = top-level, 1 = contained by another node, etc.
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EdgeView {
    pub intent_id: RecordId,
    pub source_id: String,
    pub dest_id: String,
    pub status: String,
    pub total_files: i64,
    pub completed_files: i64,
}

impl NodeView {
    /// Center X of the node in graph-layer coordinates (right edge for edges going right)
    pub fn center_x(&self) -> f64 {
        self.x + self.width / 2.0
    }
    /// Center Y of the node in graph-layer coordinates
    pub fn center_y(&self) -> f64 {
        self.y + self.height / 2.0
    }
}

/// Compute a cubic bezier path string for an edge between two points.
pub fn bezier_path(x1: f64, y1: f64, x2: f64, y2: f64) -> String {
    let dx = (x2 - x1).abs() * 0.5;
    format!(
        "M {x1} {y1} C {} {y1}, {} {y2}, {x2} {y2}",
        x1 + dx,
        x2 - dx
    )
}

/// Get the edge color based on intent status.
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

/// Check if `child` path is contained within `parent` path.
/// "/a/b/c" is contained in "/a/b" but "/a/bc" is NOT contained in "/a/b".
pub fn path_contains(parent: &str, child: &str) -> bool {
    if parent == child {
        return false;
    }
    let parent_normalized = if parent.ends_with('/') {
        parent.to_string()
    } else {
        format!("{parent}/")
    };
    child.starts_with(&parent_normalized)
}

/// Compute nesting depth for a path given a sorted list of all paths in the same container.
/// Returns 0 for top-level, 1 for paths contained by one other, etc.
pub fn compute_depth(path: &str, all_paths: &[&str]) -> usize {
    all_paths
        .iter()
        .filter(|&&other| path_contains(other, path))
        .count()
}

/// Shorten a path for display. Show last 2 components.
pub fn short_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() <= 2 {
        path.to_string()
    } else {
        format!(".../{}", parts[parts.len() - 2..].join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_contains_basic() {
        assert!(path_contains("/a/b", "/a/b/c"));
        assert!(path_contains("/a/b/", "/a/b/c"));
    }

    #[test]
    fn test_path_contains_not_prefix_trick() {
        // /a/bc is NOT contained in /a/b (no trailing slash match)
        assert!(!path_contains("/a/b", "/a/bc"));
    }

    #[test]
    fn test_path_contains_same_path() {
        assert!(!path_contains("/a/b", "/a/b"));
    }

    #[test]
    fn test_path_contains_unrelated() {
        assert!(!path_contains("/a/b", "/c/d"));
    }

    #[test]
    fn test_compute_depth() {
        let paths = vec![
            "/Users/anders/projects",
            "/Users/anders/projects/kip",
            "/Users/anders/projects/kip/src",
            "/Users/anders/music",
        ];
        assert_eq!(compute_depth("/Users/anders/projects", &paths), 0);
        assert_eq!(compute_depth("/Users/anders/projects/kip", &paths), 1);
        assert_eq!(compute_depth("/Users/anders/projects/kip/src", &paths), 2);
        assert_eq!(compute_depth("/Users/anders/music", &paths), 0);
    }
}
