# Circular Directory Nodes Implementation Plan

## Current Status

Most of the circular directory nodes functionality is already implemented in Kip. The following components are already in place:

1. NodeView data structure with required fields (`is_dir`, `is_expanded`, `is_orbit`, `child_count`)
2. Expansion state management using Dioxus signals
3. Directory detection logic in `load_nodes` function
4. Click handling for directory nodes to toggle expansion states
5. Circle rendering for directories with different states
6. Orbit view implementation with child positioning
7. Expanded view implementation

## Missing Components

### 1. Missing CSS Styles for Expanded View
**File:** `assets/main.css`

The CSS classes referenced in the design document are missing from the main stylesheet:
- `.node-expanded-container`
- `.node-expanded-header`
- `.node-expanded-body`
- `.node-expand-toggle`

### 2. Child Count Population
**File:** `src/ui/graph.rs`

The `child_count` field in `NodeView` is not being populated during the `load_nodes` function.

### 3. Orbit View Positioning Issues
**File:** `src/ui/graph.rs`

The orbit view positioning calculations need refinement for proper child placement.

### 4. Visual Styling Improvements
**Files:** `assets/main.css`, `src/ui/graph.rs`

The visual styling of orbit children needs improvement to match the design specification.

## Implementation Steps

### Step 1: Add Missing CSS Styles
Add the missing CSS classes to `assets/main.css` as specified in the design document.

### Step 2: Populate Child Count
Modify the `load_nodes` function to calculate and populate the `child_count` field for directory nodes.

### Step 3: Fix Orbit View Positioning
Refine the orbit view positioning calculations to ensure proper child placement around parent nodes.

### Step 4: Improve Visual Styling
Enhance the visual styling of orbit children and expanded views to match the design specification.

### Step 5: Testing and Integration
Test all directory node interactions and verify integration with existing graph functionality.

## Detailed Implementation

### 1. CSS Styles Addition
Add the following CSS rules to `assets/main.css`:

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

### 2. Child Count Population
Modify the `load_nodes` function to calculate child counts:

```rust
// After populating nodes vector, calculate child counts
let mut child_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

// Count children for each node
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

### 3. Orbit View Positioning Fixes
Improve the orbit view positioning calculations in the rendering code.

### 4. Visual Styling Improvements
Enhance the visual appearance of orbit children and expanded views to match the design specification.

## Testing Plan

1. Test directory node click interactions (collapsed → orbit → expanded → collapsed)
2. Verify orbit view child positioning
3. Test expanded view rendering
4. Verify integration with existing graph functionality
5. Test with various directory structures and nesting levels
6. Verify performance with large numbers of children

## Expected Outcomes

After implementing these changes, users will be able to:
1. Click directory nodes to toggle between collapsed, orbit, and expanded views
2. See children arranged in a circular orbit around parent directories
3. Expand directories to view children in a traditional list view
4. Have a visually consistent and polished user experience