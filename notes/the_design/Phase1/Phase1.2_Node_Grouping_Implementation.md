# Node Grouping Implementation

## Parent Task: Phase 1.2 Node Grouping System

This document details the implementation of the node grouping functionality within the Kip file transfer orchestrator.

## Overview

The grouping system allows users to select multiple nodes and group them into a single container node. This reduces visual clutter and creates a hierarchical organization system.

## Core Functionality

### Group Creation
- User selects multiple nodes (using Shift+click or lasso selection)
- User triggers grouping action (Ctrl+G or context menu)
- Selected nodes collapse into a single group node
- Group node displays summary information (name, member count, aggregate status)

### Group Representation
- Groups render as circular nodes (same as directories)
- Group label shows member count and summary
- Group node uses a distinct visual style from directory nodes
- Group maintains parent machine/drive color tinting

### Group Expansion
- Single click: Orbit view - members fan out around group in circular formation
- Double click: Enter view - workspace shows only group members as full nodes
- Back navigation returns to previous view

### Edge Handling
- Edges connected to any member node get consolidated to connect to the group
- When group expands, individual edges are revealed
- Edge status reflects aggregate status of member connections

## Data Model

### Database Schema
```
DEFINE TABLE node_group SCHEMAFULL;
DEFINE FIELD name ON node_group TYPE string;
DEFINE FIELD members ON node_group TYPE array<record<location>>;
DEFINE FIELD parent_group ON node_group TYPE option<record<node_group>>;
DEFINE FIELD collapsed ON node_group TYPE bool DEFAULT true;
DEFINE FIELD created_at ON node_group TYPE datetime;
```

### View Model
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct GroupView {
    pub id: RecordId,
    pub name: String,
    pub members: Vec<RecordId>,
    pub container_id: String,  // Parent machine/drive ID
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub is_expanded: bool,
    pub is_orbit: bool,
    pub member_count: usize,
    pub aggregate_status: String,  // Combined status of all members
}
```

## Implementation Details

### Group Creation Process
1. User selects multiple nodes
2. System validates selection (all nodes must be in same container)
3. Create group record in database
4. Update UI to show group node instead of individual nodes
5. Consolidate edges to/from group members

### Edge Consolidation
When nodes are grouped, their edges consolidate:
- Multiple edges to same destination → single edge from group to destination
- Edge status = worst status among member edges
- Edge count = sum of member edge counts

### Expansion States
- **Collapsed**: Single group node showing summary
- **Orbit**: Members fanned out around group in circular formation
- **Expanded**: Workspace shows only group members as full nodes

## UI/UX Considerations

### Visual Hierarchy
- Group nodes visually distinct from directory nodes
- Clear affordances for expansion states
- Smooth animations between states
- Consistent glassmorphic styling

### Interaction Model
- Keyboard shortcut (Ctrl+G) for grouping
- Context menu option for grouping
- Drag-and-drop support for creating groups
- Undo/redo support for grouping operations

## Technical Challenges

### Coordinate Management
- Proper positioning of grouped nodes
- Edge routing around group boundaries
- Animation between collapsed/expanded states

### State Synchronization
- Keep UI state in sync with database
- Handle concurrent modifications
- Maintain selection state across grouping operations

## Success Criteria

- [ ] Users can select multiple nodes and create groups
- [ ] Groups display properly with summary information
- [ ] Edge consolidation works correctly
- [ ] Expansion states function as expected
- [ ] Groups persist across application restarts
- [ ] Performance remains acceptable with nested groups