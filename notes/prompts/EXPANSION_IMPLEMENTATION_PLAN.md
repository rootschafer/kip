# Directory Expansion Implementation Plan

## Overview
This plan outlines the complete implementation of directory expansion functionality (orbit and enter views) for the Kip file transfer orchestrator.

## Current State
- Directory nodes can be clicked to toggle expansion states
- Expansion state is tracked in a HashMap signal
- Visual styling exists for different states
- Root folder nodes have been added for each drive

## Challenges to Address
1. **Dynamic node positioning**: Child nodes need to be repositioned when parent enters orbit state
2. **Node visibility**: Nodes need to be hidden/shown based on expansion state
3. **Context switching**: When entering a directory, only its direct children should be visible

## Implementation Strategy

### Phase 1: Enhanced Data Structures
1. **Update NodeView** to include parent reference and full path
2. **Create helper functions** to determine node relationships and visibility
3. **Modify expansion state tracking** to handle complex hierarchies

### Phase 2: Dynamic Positioning Logic
1. **Implement orbit positioning**: Calculate positions for children around parent when in orbit state
2. **Implement enter positioning**: Position direct children appropriately when in enter state
3. **Handle transitions**: Smooth animations between states

### Phase 3: Visibility Management
1. **Filter nodes** based on current view context (collapsed, orbit, enter)
2. **Update rendering logic** to show/hide nodes based on expansion state
3. **Maintain selection state** across view changes

### Phase 4: UI/UX Implementation
1. **Update WorkspaceNode** to respond to dynamic positioning
2. **Implement smooth transitions** between states
3. **Add visual indicators** for current view context
4. **Update edge rendering** to work with dynamic node positions

## Technical Approach

### Node Relationships
```rust
// Enhanced NodeView structure
pub struct NodeView {
    // existing fields...
    pub parent_path: Option<String>,  // Path of parent directory
    pub full_path: String,            // Full path from root
    pub depth: usize,                 // Depth in the hierarchy
    pub total_descendants: usize,     // Total count of all descendants (recursive)
}
```

### Positioning Functions
- `calculate_orbit_positions(parent_node: &NodeView, children: &[NodeView]) -> Vec<(String, f64, f64)>`
- `calculate_enter_positions(parent_node: &NodeView, children: &[NodeView]) -> Vec<(String, f64, f64)>`
- `get_visible_nodes(all_nodes: &[NodeView], expansion_state: &HashMap<String, (bool, bool)>) -> Vec<NodeView>`
- `calculate_node_size(total_descendants: usize, total_workspace_nodes: usize) -> f64` (uses logarithmic scaling)

### Animation Functions
- Use CSS transitions for radial movement in orbit state
- Implement smooth transitions between states

### State Management
- Track current "entered" directory separately from orbit/expanded states
- Implement navigation history for back button functionality
- Handle edge cases like expanding a node that's not visible in current context

## Implementation Steps

### Step 1: Update Data Structures
- Modify `load_nodes` to establish parent-child relationships
- Add helper functions to determine node relationships
- Update NodeView struct with parent and full path information

### Step 2: Implement Positioning Logic
- Create functions to calculate orbit positions around parent nodes
- Create functions to calculate positions for enter view
- Implement smooth transition animations

### Step 3: Update Rendering Pipeline
- Modify the main graph component to filter nodes based on current view
- Update node positioning based on expansion state
- Ensure edges are rendered correctly with dynamic node positions

### Step 4: Enhance User Experience
- Add breadcrumbs for navigation
- Implement back button functionality
- Add visual feedback for expansion states
- Ensure smooth animations between states

## Expected Outcomes
- Users can click directory nodes to see children in orbit view
- Users can double-click to enter a directory and see only its direct children
- Users can navigate back up the hierarchy
- Smooth transitions between different view states
- Children animate moving out radially when parent enters orbit state
- Directory/group node size scales based on total child count (all descendants, not just direct children)
- Node sizing uses logarithmic scaling to prevent extreme sizes
- Proper visual feedback for current context

## Risks & Mitigations
- **Performance**: Large directories may impact performance - implement virtualization if needed
- **Complexity**: State management could become complex - use clear separation of concerns
- **Edge cases**: Multiple expansion states simultaneously - ensure proper state isolation

## Success Criteria
- Directory orbit view shows children properly positioned around parent
- Directory enter view shows only direct children of the entered directory
- Navigation between states is smooth and intuitive
- Back navigation works properly
- Performance remains acceptable with moderate directory sizes (<100 direct children)