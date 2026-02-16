# Phase 1.3: Force-Directed Layout Implementation

**Status:** NOT IMPLEMENTED
**Priority:** HIGH
**Estimated Effort:** 4-6 hours

---

## Overview

Replace the current static grid layout with a force-directed graph that automatically arranges nodes based on their relationships. Nodes should settle into a visually pleasing layout that minimizes edge crossings and maintains appropriate spacing.

---

## Reference Implementation

**Location:** `external/nexus-node-sync/components/GraphCanvas.tsx`

The TypeScript implementation uses D3.js force simulation with these forces:

```typescript
d3.forceSimulation(nodes)
  .force("link", d3.forceLink(links)
    .id(d => d.id)
    .distance(d => d.type === 'sync' ? 150 : 80))
  .force("charge", d3.forceManyBody().strength(-300))
  .force("x", d3.forceX().strength(0.04))
  .force("y", d3.forceY().strength(0.04))
  .force("collide", d3.forceCollide()
    .radius(d => d.type === 'device' ? 45 : 30)
    .iterations(2))
```

---

## Requirements

### Physics Forces

1. **Repulsion (forceManyBody)**
   - All nodes push each other apart
   - Strength: -300 (negative = repulsion)
   - Prevents node overlap
   - Helps spread nodes evenly

2. **Link Attraction (forceLink)**
   - Connected nodes pull toward each other
   - Distance varies by edge type:
     - Sync edges: 150px (longer, emphasizes relationship)
     - Hierarchy edges: 80px (shorter, keeps parent/child close)
     - Group edges: 60px (very short, keeps group tight)

3. **Center Gravity (forceX/forceY)**
   - Gentle pull toward center of canvas
   - Strength: 0.04 (very weak)
   - Prevents nodes from flying off screen
   - Creates organic clustering

4. **Collision (forceCollide)**
   - Prevents nodes from overlapping
   - Radius based on node type:
     - Device nodes: 45px
     - Folder/Group nodes: 30px
     - File nodes: 15px
   - Iterations: 2 (smoother collision resolution)

### Simulation Control

1. **Start Conditions**
   - Start when nodes are added
   - Start when layout is requested
   - Start with alpha (energy) = 0.5

2. **Stop Conditions**
   - Stop when alpha < 0.001 (settled)
   - Stop after max iterations (safety)
   - Stop when user pauses layout

3. **Restart Conditions**
   - Node added/removed
   - Edge added/removed
   - User triggers re-layout
   - Alpha decayed too low

### Node Dragging

1. **During Drag**
   - Fix node position (fx, fy)
   - Simulation continues for other nodes
   - Connected nodes follow naturally

2. **After Drag**
   - Release position (fx = null, fy = null)
   - Node rejoins simulation
   - Settles into new position

3. **Multi-Node Drag**
   - Drag all selected nodes together
   - Maintain relative positions
   - All fixed during drag, released after

### Animation

1. **Node Entry**
   - Fade in opacity: 0 → 1
   - Scale up: 0 → 1
   - Duration: 300-500ms

2. **Layout Transition**
   - Smooth position updates (lerp)
   - Duration based on distance
   - Typical: 500-1000ms

3. **Expansion Animation**
   - Children emerge from parent
   - Orbit outward to final positions
   - Duration: 500-800ms

---

## Implementation Plan

### Step 1: Choose Physics Library

**Option A: Use Existing Rust Crate**
- `force-graph` - D3-like API
- `graphology` - Graph algorithms + layout
- `petgraph` + custom forces - More control

**Option B: Port D3 Logic**
- Direct translation of D3 force simulation
- More control, more work
- Better match with reference implementation

**Recommendation:** Option B for exact behavior match

### Step 2: Implement Simulation Loop

```rust
// In src/ui/graph_store.rs

impl Graph {
    pub fn start_simulation(&mut self) {
        self.sim_running = true;
        self.alpha = 0.5; // Start with medium energy
    }

    pub fn stop_simulation(&mut self) {
        self.sim_running = false;
        self.alpha = 0.0;
    }

    pub fn tick(&mut self) -> bool {
        if !self.sim_running {
            return false;
        }

        // Apply forces
        self.apply_repulsion();
        self.apply_link_forces();
        self.apply_center_gravity();
        self.resolve_collisions();

        // Update positions
        self.integrate();

        // Decay alpha
        self.alpha *= 0.99; // ALPHA_DECAY

        // Check if settled
        if self.alpha < 0.001 {
            self.stop_simulation();
            return false;
        }

        true
    }
}
```

### Step 3: Add Effect Loop

```rust
// In src/ui/graph.rs

use_effect(move || {
    let graph_signal = graph;
    spawn(async move {
        loop {
            let should_run = graph_signal.with(|g| g.sim_running);
            if !should_run {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            // Tick at ~60fps
            tokio::time::sleep(Duration::from_millis(16)).await;

            graph_signal.with_mut(|g| g.tick());
        }
    });
});
```

**CRITICAL:** Ensure this doesn't create infinite loops:
- Don't capture signals incorrectly
- Don't trigger state updates in tick
- Use proper dependency tracking

### Step 4: Implement Forces

**Repulsion:**
```rust
fn apply_repulsion(&mut self) {
    for i in 0..self.nodes.len() {
        for j in (i + 1)..self.nodes.len() {
            let dx = self.nodes[i].x - self.nodes[j].x;
            let dy = self.nodes[i].y - self.nodes[j].y;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let force = REPULSION / (dist * dist);
            let fx = (dx / dist) * force;
            let fy = (dy / dist) * force;

            self.nodes[i].vx += fx;
            self.nodes[i].vy += fy;
            self.nodes[j].vx -= fx;
            self.nodes[j].vy -= fy;
        }
    }
}
```

**Link Attraction:**
```rust
fn apply_link_forces(&mut self) {
    for edge in &self.edges {
        let source = self.find_node(&edge.source_id);
        let target = self.find_node(&edge.dest_id);
        if let (Some(s), Some(t)) = (source, target) {
            let dx = t.x - s.x;
            let dy = t.y - s.y;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let target_dist = self.get_edge_length(edge);
            let force = (dist - target_dist) * SPRING_K;
            let fx = (dx / dist) * force;
            let fy = (dy / dist) * force;

            s.vx += fx;
            s.vy += fy;
            t.vx -= fx;
            t.vy -= fy;
        }
    }
}
```

### Step 5: Add Drag Support

```rust
// In mouse drag handler
fn drag_started(&mut self, node_id: &str) {
    if let Some(node) = self.find_node_mut(node_id) {
        node.fx = Some(node.x);
        node.fy = Some(node.y);
    }
}

fn drag_moved(&mut self, node_id: &str, x: f64, y: f64) {
    if let Some(node) = self.find_node_mut(node_id) {
        node.fx = Some(x);
        node.fy = Some(y);
        node.x = x;
        node.y = y;
    }
}

fn drag_ended(&mut self, node_id: &str) {
    if let Some(node) = self.find_node_mut(node_id) {
        node.fx = None;
        node.fy = None;
    }
    // Restart simulation to settle
    self.start_simulation();
}
```

### Step 6: Visual Polish

1. **Node Sizing**
   ```rust
   fn calculate_node_size(descendants: usize) -> f64 {
       let log_count = (1.0 + descendants as f64).ln();
       (80.0 + log_count * 15.0).clamp(60.0, 150.0)
   }
   ```

2. **Edge Lengths**
   ```rust
   fn get_edge_length(&self, edge: &GraphEdge) -> f64 {
       match edge.status {
           "sync" => 150.0,
           "group" => 60.0,
           _ => 80.0,
       }
   }
   ```

3. **Collision Radius**
   ```rust
   fn get_collision_radius(&self, node: &GraphNode) -> f64 {
       match node.kind {
           NodeKind::Device => 45.0,
           NodeKind::Directory { .. } | NodeKind::Group { .. } => 30.0,
           NodeKind::File => 15.0,
       }
   }
   ```

---

## Testing Checklist

- [ ] Nodes spread out without overlapping
- [ ] Connected nodes stay close together
- [ ] Graph stays centered on canvas
- [ ] Drag feels responsive
- [ ] Multi-node drag works
- [ ] Layout settles (doesn't oscillate forever)
- [ ] Works with 5 nodes
- [ ] Works with 50 nodes
- [ ] Works with 200 nodes (performance check)
- [ ] No infinite loops (CPU stays reasonable)
- [ ] No memory leaks (memory stable over time)

---

## Common Pitfalls

### 1. Simulation Never Settles
**Cause:** Alpha decay too slow or forces too strong
**Fix:** Increase ALPHA_DECAY (0.99 → 0.95) or reduce force strengths

### 2. Nodes Fly Off Screen
**Cause:** Center gravity too weak or repulsion too strong
**Fix:** Increase center force strength or reduce repulsion

### 3. Infinite Loop in Tick
**Cause:** Tick triggers state update which triggers tick again
**Fix:** Don't update signals from inside tick(), use separate effect

### 4. Drag Doesn't Work
**Cause:** Fixed position (fx/fy) not respected in integration
**Fix:** Skip integration for nodes with fx/fy set

### 5. Performance Issues
**Cause:** O(n²) repulsion calculation for many nodes
**Fix:** Use Barnes-Hut approximation or limit repulsion to nearby nodes

---

## Success Criteria

1. **Visual:** Graph looks organized, not random
2. **Interactive:** Drag feels smooth, responsive
3. **Stable:** Layout settles, doesn't jitter
4. **Performant:** 60fps with 100 nodes
5. **Stable:** No crashes, no infinite loops

---

## Related Files

- `src/ui/graph_store.rs` - Graph struct, simulation logic
- `src/ui/graph.rs` - Component, effect loop
- `src/ui/graph_nodes.rs` - Node rendering
- `src/ui/graph_edges.rs` - Edge rendering
- `src/ui/graph_types.rs` - Node/Edge types

---

## Notes

- Start with basic forces (repulsion + link)
- Add complexity gradually (center, collision)
- Test after each addition
- Keep TypeScript reference open for comparison
- Watch for infinite loops (check CPU frequently)
