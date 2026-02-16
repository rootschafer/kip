# Immediate Next Steps Implementation Plan

## 1. Complete Circular Directory Nodes Feature

### 1.1. Add Missing CSS Styles
**File**: `assets/main.css`

Add the missing CSS classes for the expanded view of circular nodes:

```css
/* ─── Expanded view for circular nodes ─── */
.node-expanded-container {
    position: absolute;
    background: var(--glass-strong);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius);
    padding: 12px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
    z-index: 100;
}

.node-expanded-header {
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
    color: var(--text);
}

.node-expanded-body {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.node-expand-toggle {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--glass-hover);
    border: 1px solid var(--glass-border);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 10px;
    transition: all 0.15s ease;
}

.node-expand-toggle:hover {
    background: var(--accent);
    border-color: var(--accent);
}

.expanded-child-row {
    display: flex;
    align-items: center;
    padding: 4px 8px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s ease;
}

.expanded-child-row:hover {
    background: var(--glass-hover);
}

.expanded-child {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
}

.expanded-child-check {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    background: var(--accent);
    color: white;
    font-size: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
}

.expanded-child-label {
    font-size: 12px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
}
```

### 1.2. Populate Child Count in load_nodes
**File**: `src/ui/graph.rs`

Modify the `load_nodes` function to calculate and populate the `child_count` field:

```rust
// After populating nodes vector, calculate child counts
let mut child_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

// Initialize child counts for all directory nodes
for node in &nodes {
    if node.is_dir {
        let node_id = rid_string(&node.id);
        child_counts.insert(node_id, 0);
    }
}

// Calculate direct children for each directory
for node in &nodes {
    if !node.is_dir {
        continue;
    }
    
    let node_path = &node.path;
    let prefix = if node_path.ends_with('/') {
        node_path.clone()
    } else {
        format!("{}/", node_path)
    };
    
    let mut count = 0;
    for other_node in &nodes {
        if other_node.container_id == node.container_id &&
           other_node.path.starts_with(&prefix) &&
           other_node.path != *node_path &&
           other_node.depth == node.depth + 1 {
            count += 1;
        }
    }
    
    let node_id = rid_string(&node.id);
    child_counts.insert(node_id, count);
}

// Update nodes with child counts
for node in &mut nodes {
    if node.is_dir {
        let node_id = rid_string(&node.id);
        if let Some(count) = child_counts.get(&node_id) {
            node.child_count = *count;
        }
    }
}
```

### 1.3. Fix Orbit View Positioning
**File**: `src/ui/graph.rs`

Improve the orbit view positioning calculations in the rendering code.

### 1.4. Improve Visual Styling
**Files**: `assets/main.css`, `src/ui/graph.rs`

Enhance the visual appearance of orbit children and expanded views.

## 2. Implement Grouping Feature

### 2.1. Add NodeGroup Table to Database Schema
**File**: `src/db.rs`

Add the node_group table definition to the schema:

```surql
DEFINE TABLE OVERWRITE node_group SCHEMAFULL;
DEFINE FIELD OVERWRITE name ON node_group TYPE string;
DEFINE FIELD OVERWRITE members ON node_group TYPE array<record<location>>;
DEFINE FIELD OVERWRITE parent_group ON node_group TYPE option<record<node_group>>;
DEFINE FIELD OVERWRITE collapsed ON node_group TYPE bool DEFAULT true;
DEFINE FIELD OVERWRITE created_at ON node_group TYPE datetime;
```

### 2.2. Create Group Data Models
**File**: `src/models/mod.rs`

Add a new module for group models:

```rust
// src/models/group.rs
use surrealdb::types::RecordId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGroup {
    pub id: RecordId,
    pub name: String,
    pub members: Vec<RecordId>,
    pub parent_group: Option<RecordId>,
    pub collapsed: bool,
    pub created_at: String,
}
```

### 2.3. Update Graph Data Loading
**File**: `src/ui/graph.rs`

Modify the graph data loading to include groups:

1. Add GroupView data structure to `graph_types.rs`
2. Update `load_graph_data` function to load groups
3. Modify rendering logic to handle groups alongside nodes

### 2.4. Implement Group Rendering
**File**: `src/ui/graph.rs`

Implement group rendering similar to directory nodes:
- Groups render as circles
- Click once for orbit view
- Click again for enter view
- Show member count in collapsed view

### 2.5. Add Group Creation UI
**File**: `src/ui/graph.rs`

Add UI for creating groups:
- Context menu on selected nodes
- Toolbar button for grouping
- Group naming dialog

### 2.6. Implement Edge Merging Logic
**File**: `src/ui/graph.rs`

Implement logic to merge edges when nodes are grouped:
- Find edges connected to group members
- Merge them into a single edge to/from the group
- Show aggregate status in merged edges

### 2.7. Add Group Management Features
**File**: `src/ui/graph.rs`

Implement group management:
- Rename groups
- Add/remove members
- Delete groups
- Nested group support

## 3. Add Central Output Node

### 3.1. Define Output Node Data Structure
**File**: `src/ui/graph_types.rs`

Add a special OutputNodeView structure or extend ContainerView.

### 3.2. Implement Output Node Rendering
**File**: `src/ui/graph.rs`

Render the Output node in the center of the workspace with special styling.

### 3.3. Add Output Node Interactions
**File**: `src/ui/graph.rs`

Enable connecting nodes to the Output node and handle edge creation.

## 4. Add Per-Node Error Badges

### 4.1. Update Node Data Loading
**File**: `src/ui/graph.rs`

Modify `load_nodes` to include error counts from review items.

### 4.2. Add Error Badge Rendering
**File**: `src/ui/graph.rs`

Render error badges on nodes with unresolved review items.

### 4.3. Implement Badge Interactions
**File**: `src/ui/graph.rs`

Add click handlers to scroll the review queue to relevant items.

## Implementation Timeline

### Week 1: Complete Circular Directory Nodes
- Add missing CSS styles
- Populate child counts
- Fix orbit view positioning
- Improve visual styling
- Test and refine

### Week 2: Begin Grouping Implementation
- Add database schema
- Create data models
- Update graph data loading
- Implement basic group rendering

### Week 3: Complete Grouping Feature
- Add group creation UI
- Implement edge merging
- Add group management features
- Test grouping workflows

### Week 4: Additional Features
- Implement Output node
- Add error badges
- Integration testing
- Documentation updates

## Testing Strategy

### Unit Tests
- Test child count calculation logic
- Test group membership operations
- Test edge merging algorithms

### Integration Tests
- Test full grouping workflow
- Test Output node interactions
- Test error badge functionality

### Manual Testing
- Verify visual appearance of all node types
- Test expansion states for directories and groups
- Validate edge merging behavior
- Check error badge display and interactions

## Success Criteria

### Circular Directory Nodes
- Directory nodes render as circles
- Click once shows orbit view
- Click twice shows expanded view
- Child counts display correctly
- Visual styling matches design specification

### Grouping Feature
- Users can select multiple nodes and create groups
- Groups render as circles with member counts
- Edge merging works correctly
- Group management functions properly
- Nested groups supported

### Output Node
- Output node renders in center of workspace
- Users can connect nodes to Output
- Visual styling is distinct and appealing

### Error Badges
- Error badges display on nodes with issues
- Badge counts are accurate
- Clicking badges navigates to review queue
- Visual styling is clear and noticeable