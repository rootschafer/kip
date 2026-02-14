# Force-Directed Layout Engine Implementation

## Parent Task: Phase 1.3 Force-Directed Layout Engine

This document details the implementation of the physics-based force-directed layout system for the Kip workspace.

## Overview

The force-directed layout engine replaces the temporary grid positioning with a physics simulation that arranges nodes based on attractive and repulsive forces. This creates organic, readable layouts that minimize edge crossings and cluster related nodes.

## Core Physics Model

### Forces Applied

#### Node Repulsion
- All nodes repel each other with force inversely proportional to distance
- Prevents node overlap and improves readability
- Formula: `F = k_repulse / distance²`

#### Edge Attraction
- Connected nodes attract each other with force proportional to distance
- Keeps related nodes close together
- Formula: `F = k_attract * distance`

#### Container Cohesion
- Nodes belonging to the same machine/drive cluster together
- Uses color-based attraction (nodes with same --node-color attract)
- Helps maintain logical groupings

#### Cross-Crossing Minimization
- Additional force to reduce visual clutter from crossing edges
- Applies repulsion along edge paths to push them apart

### Layout Algorithm

Uses a variation of the Fruchterman-Reingold algorithm:
1. Initialize nodes with random positions
2. For each iteration:
   - Calculate repulsive forces between all node pairs
   - Calculate attractive forces between connected nodes
   - Apply forces to update positions
   - Apply boundary constraints
3. Repeat until convergence or max iterations

## Implementation Details

### Data Structures

```rust
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub vx: f64,  // Velocity x
    pub vy: f64,  // Velocity y
    pub mass: f64,
    pub container_id: String,
    pub is_fixed: bool,  // For user-dragged nodes
}

#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub source_id: String,
    pub dest_id: String,
    pub weight: f64,  // For weighted attraction
}
```

### Configuration Parameters

```rust
const REPULSION_STRENGTH: f64 = 1000.0;
const ATTRACTION_STRENGTH: f64 = 0.1;
const COHESION_STRENGTH: f64 = 0.05;
const TIME_STEP: f64 = 0.05;
const MAX_ITERATIONS: usize = 100;
const CONVERGENCE_THRESHOLD: f64 = 0.1;
```

### Layout Process

1. **Initialization**: Place nodes randomly or use existing positions as seed
2. **Force Calculation**: Compute all forces for current positions
3. **Position Update**: Apply forces with damping
4. **Constraint Application**: Apply boundary and user-fixed constraints
5. **Iteration**: Repeat until convergence

## User Interaction

### Pinning
- User can drag nodes to pin them at specific positions
- Pinned nodes become `is_fixed = true` and don't participate in physics
- Layout algorithm works around pinned positions

### Layout Persistence
- Final positions saved to database
- Layout restored on application restart
- User preferences for pinned positions preserved

## Performance Considerations

### Optimization Strategies
- Spatial partitioning (quadtree) to optimize repulsion calculations
- Parallel computation of forces using Rayon
- Incremental updates when nodes are added/removed
- Adaptive time stepping for stability

### Complexity
- Naive: O(N² + E) per iteration
- With quadtree: O(N log N + E) per iteration
- Target: <100ms per layout iteration for 1000 nodes

## Integration Points

### With Node System
- Layout engine provides positions for all visible nodes
- Works with expansion states (orbit/enter views)
- Handles dynamic node addition/removal

### With Database
- Reads/writes positions to location table
- Persists user preferences for pinned nodes
- Handles layout restoration on startup

## Success Criteria

- [ ] Force-directed layout algorithm implemented and stable
- [ ] Physics parameters tuned for good visual results
- [ ] Performance acceptable for 1000+ nodes
- [ ] User pinning functionality works
- [ ] Layout persists across application restarts
- [ ] Integrates properly with expansion states
- [ ] Edge crossings minimized
- [ ] Related nodes clustered appropriately