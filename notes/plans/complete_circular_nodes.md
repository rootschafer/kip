# Complete Circular Directory Nodes Implementation

## Current Status

The circular directory nodes feature is partially implemented with the following components already in place:

1. NodeView data structure with required fields (`is_dir`, `is_expanded`, `is_orbit`, `child_count`)
2. Expansion state management using Dioxus signals
3. Directory detection logic in `load_nodes` function
4. Click handling for directory nodes to toggle expansion states
5. Circle rendering for directories with different states
6. Orbit view implementation with child positioning
7. Expanded view implementation

## Missing Components

### 1. Missing CSS Styles for Expanded View
**File**: `assets/main.css`

The CSS classes referenced in the design document are missing from the main stylesheet:
- `.node-expanded-container`
- `.node-expanded-header`
- `.node-expanded-body`
- `.node-expand-toggle`
- Related classes for child rows and labels

### 2. Child Count Population
**File**: `src/ui/graph.rs`

The `child_count` field in `NodeView` is not being populated during the `load_nodes` function.

### 3. Orbit View Positioning Issues
**File**: `src/ui/graph.rs`

The orbit view positioning calculations need refinement for proper child placement.

### 4. Visual Styling Improvements
**Files**: `assets/main.css`, `src/ui/graph.rs`

The visual styling of orbit children needs improvement to match the design specification.

## Detailed Implementation Plan

### Phase 1: Add Missing CSS Styles (1-2 hours)

**Task**: Add the missing CSS classes to `assets/main.css`

**Implementation**:
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

### Phase 2: Populate Child Count (2-3 hours)

**Task**: Modify the `load_nodes` function to calculate and populate the `child_count` field

**Implementation**:
1. After populating the nodes vector, iterate through all nodes to calculate child counts
2. For each directory node, count its direct children based on path containment
3. Update each node's `child_count` field with the calculated value

**Code Changes**:
```rust
// In load_nodes function, after the main node population loop

// Calculate child counts for directory nodes
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
        // Check if other_node is a direct child of node
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

### Phase 3: Fix Orbit View Positioning (2-3 hours)

**Task**: Refine the orbit view positioning calculations for proper child placement

**Implementation**:
1. Review the current `compute_orbit_positions` function in `graph_types.rs`
2. Ensure the positioning algorithm correctly places children in a circular arrangement
3. Adjust the positioning calculations in the orbit view rendering code in `graph.rs`

### Phase 4: Improve Visual Styling (1-2 hours)

**Task**: Enhance the visual styling of orbit children and expanded views

**Implementation**:
1. Review the current CSS for orbit children in `assets/main.css`
2. Add or improve styles for better visual appearance
3. Ensure consistent styling with the overall glassmorphic design

## Testing Plan

### Unit Testing
- Test child count calculation logic with various directory structures
- Verify orbit positioning calculations with different numbers of children

### Integration Testing
- Test directory node click interactions (collapsed → orbit → expanded → collapsed)
- Verify orbit view child positioning with multiple directories
- Test expanded view rendering with nested directories
- Verify integration with existing graph functionality

### Manual Testing
- Visually inspect all node states (collapsed, orbit, expanded)
- Test with various directory structures and nesting levels
- Verify performance with large numbers of children
- Check visual consistency with the overall design

## Success Criteria

1. **CSS Styles**: All required CSS classes are added and properly styled
2. **Child Counts**: Directory nodes display accurate child counts
3. **Orbit View**: Children are properly positioned in a circular arrangement around parent nodes
4. **Expanded View**: Directory contents are displayed correctly when expanded
5. **Visual Consistency**: All visual elements match the glassmorphic design language
6. **Functionality**: All expansion states work correctly (click once for orbit, click again for expand)
7. **Performance**: No significant performance degradation with large numbers of nodes or children

## Timeline

- **Phase 1 (CSS)**: 1-2 hours
- **Phase 2 (Child Count)**: 2-3 hours
- **Phase 3 (Positioning)**: 2-3 hours
- **Phase 4 (Styling)**: 1-2 hours
- **Testing**: 2-3 hours

**Total Estimated Time**: 8-13 hours

## Dependencies

- None - all required components are already in place
- No database schema changes required
- No new external dependencies needed

## Risks

1. **Performance**: Calculating child counts for large graphs could impact performance
   - Mitigation: Optimize the child counting algorithm
   - Mitigation: Only calculate when graph data is refreshed

2. **Visual Consistency**: New styles might not integrate well with existing design
   - Mitigation: Follow existing CSS patterns and variables
   - Mitigation: Test thoroughly with various scenarios

3. **Positioning Accuracy**: Orbit view positioning might not be pixel-perfect
   - Mitigation: Use mathematical calculations for precise positioning
   - Mitigation: Test with various numbers of children

## Rollback Plan

If issues are encountered:

1. Revert CSS changes to restore previous styling
2. Comment out child count calculation code to restore previous behavior
3. Revert any positioning adjustments to the orbit view
4. Document issues and address in subsequent iteration

## Verification Steps

1. Run `dx build` to ensure no compilation errors
2. Run `dx serve --platform desktop` to test in development mode
3. Verify all node types render correctly
4. Test click interactions for directory nodes
5. Verify child counts display accurately
6. Test with various directory structures
7. Verify no performance regressions