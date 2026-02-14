# Critical Issues Requiring Immediate Attention

## SVG Coordinate System Alignment Issue

### Problem
Mouse coordinates captured from MouseEvent don't align with SVG overlay coordinates, causing cursor offset in edge creation. The rubber-band line endpoint appears ~100px below and to the right of the actual cursor position.

### Impact
- Edge creation is inaccurate and frustrating to use
- Interferes with node expansion functionality
- Makes the entire interaction system feel broken

### Root Cause
The SVG overlay and HTML elements may have different coordinate systems. The mouse coordinates are captured relative to the page/window, but the SVG overlay might be positioned differently due to the toolbar/header.

### Solution Approach
1. Calculate the offset between window coordinates and SVG coordinates
2. Apply the offset when rendering the rubber-band line
3. Ensure consistent coordinate system across HTML and SVG layers

## Click vs Drag Detection Issue

### Problem
The current implementation doesn't properly distinguish between clicks (for expansion) and drags (for edge creation), causing interference between the two functionalities.

### Impact
- Clicking directory nodes triggers edge creation mode instead of expansion
- Users can't properly expand/collapse directory nodes
- Poor user experience for both expansion and edge creation

### Root Cause
The distance threshold approach isn't properly implemented or the event handling logic conflicts between expansion and edge creation.

### Solution Approach
1. Implement proper click vs drag detection with distance and time thresholds
2. Separate the event handling logic for expansion vs edge creation
3. Use different mouse events or modifiers for different actions

## Orbit View Implementation Issue

### Problem
Children are not properly fanned out around parent nodes in orbit state. The orbit positioning algorithm isn't correctly implemented.

### Impact
- Directory expansion doesn't show children in orbit view
- Users can't see the hierarchical structure visually
- Missing core functionality for exploring directory contents

### Root Cause
The orbit positioning calculation and rendering logic isn't properly implemented in the current codebase.

### Solution Approach
1. Implement proper orbit positioning algorithm using trigonometry
2. Update rendering to show children in circular formation around parent
3. Add smooth animations for orbit state transitions

## Enter View Implementation Issue

### Problem
When entering a directory (expanded state), the workspace doesn't properly filter to show only direct children of the entered directory.

### Impact
- Users can't navigate into directories to see their contents
- Missing core functionality for hierarchical exploration
- Workspace remains cluttered with unrelated nodes

### Root Cause
The node filtering logic in `get_visible_nodes` isn't properly implemented for the "entered" state.

### Solution Approach
1. Update `get_visible_nodes` to filter based on current "entered" directory
2. Implement proper navigation history and back button functionality
3. Add visual indicators for current directory context

## Recommended Implementation Priority
1. Fix SVG coordinate system alignment (foundational for all interactions)
2. Implement proper click vs drag detection
3. Complete orbit view functionality
4. Complete enter view functionality