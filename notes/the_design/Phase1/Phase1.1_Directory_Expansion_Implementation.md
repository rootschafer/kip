# Directory Expansion Implementation

## Parent Task: Phase 1.1 Directory Expansion System

This document details the implementation of the directory expansion functionality in Kip, which allows users to visualize and navigate directory hierarchies through orbit and enter views.

## Overview

The directory expansion system provides two ways to explore directory contents:
1. **Orbit view**: Children fan out in a circular formation around the parent directory
2. **Enter view**: Navigate into the directory to see only its direct children

## Core Functionality

### Node State Management
- **Collapsed**: Default state, node shows basic information
- **Orbit**: Children fanned out around parent (single click)
- **Entered**: Workspace shows only direct children of this directory (double click)

### Visual Representation
- Directory nodes render as circles with child count
- File nodes render as pills (rectangles)
- Node size scales logarithmically with total descendant count
- Orbit view shows children positioned in a ring around parent
- Enter view filters workspace to show only direct children

### Interaction Model
- **Single click on directory**: Enter orbit state (children fan out around parent)
- **Double click on directory**: Enter expanded state (navigate into directory)
- **Click on back button**: Exit expanded state (return to parent view)

## Implementation Details

### Expansion State Tracking
```rust
// Expansion state: (is_orbit, is_expanded)
// (false, false) = collapsed
// (true, false) = orbit (children fanned out)
// (false, true) = expanded (entered view, showing only direct children)
let expansion_state = use_signal(|| HashMap::<String, (bool, bool)>::new());
```

### Node Sizing Algorithm
```rust
fn calculate_node_size(total_descendants: usize, _total_workspace_nodes: usize) -> f64 {
    // Apply logarithmic scaling to the descendant count directly
    // Using log(1 + x) to scale the count, then transform to appropriate size range
    let log_count = (1.0 + total_descendants as f64).ln();
    
    // Transform to pixel size: base size + contribution from log of descendants
    let calculated_size = 40.0 + (log_count * 10.0); // Base 40px + contribution from log count
    
    // Clamp to reasonable min/max values
    calculated_size.clamp(40.0, 120.0) // Minimum 40px (so text is readable), maximum 120px
}
```

### Orbit Positioning
```rust
fn calculate_orbit_positions(
    parent_x: f64,
    parent_y: f64,
    children: &[&NodeView],
    radius: f64,
) -> Vec<(String, f64, f64)> {
    if children.is_empty() {
        return Vec::new();
    }
    
    let angle_increment = 2.0 * std::f64::consts::PI / children.len() as f64;
    
    children
        .iter()
        .enumerate()
        .map(|(i, child)| {
            let angle = i as f64 * angle_increment;
            let x = parent_x + radius * angle.cos();
            let y = parent_y + radius * angle.sin();
            (rid_string(&child.id), x, y)
        })
        .collect()
}
```

### Enter View Filtering
When a directory is in "entered" state, only show its direct children:
- Find the currently expanded directory
- Filter all nodes to show only direct children of that directory
- Calculate positions for direct children in the workspace grid

## Data Flow

### Total Descendant Calculation
1. Load all location paths from database
2. For each directory node, count all paths that are descendants
3. Use logarithmic scaling to determine node size
4. Store descendant count in NodeView struct

### Node Visibility Filtering
1. Check expansion state for currently "entered" directory
2. If in entered state, filter nodes to show only direct children
3. If in orbit state, position children around parent
4. If in collapsed state, show all nodes normally

## UI Components

### WorkspaceNode Component
- Handles click vs drag detection (distance threshold < 5px = click)
- Updates expansion state on directory clicks
- Renders differently based on expansion state
- Manages visual feedback for different states

### GraphToolbar Component
- Shows back button when in "entered" view
- Handles navigation back to parent directory
- Provides context for current view

## Integration Points

### With Database
- Load location paths to calculate descendant counts
- Update positions when entering/expanding directories
- Persist layout positions to database

### With File Picker
- Connect directory expansion to file picker functionality
- Share path information between components
- Maintain consistent navigation state

## Success Criteria

- [ ] Single click on directory shows children in orbit view
- [ ] Double click on directory enters directory (showing only direct children)
- [ ] Back button returns to parent view
- [ ] Node sizes scale appropriately with content
- [ ] Visual feedback clearly indicates expansion states
- [ ] Performance remains acceptable with large directory trees
- [ ] Proper click vs drag detection (avoid accidental edge creation)
- [ ] Smooth transitions between states