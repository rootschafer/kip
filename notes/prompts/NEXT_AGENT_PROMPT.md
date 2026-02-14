# Next Agent Prompt: Kip Development Continuation

## Project Context

Kip is a file transfer orchestrator built with Rust/Dioxus/SurrealDB that provides a visual interface for managing file synchronization between machines and drives. The application has undergone a major architectural redesign from container-nested nodes to a free workspace layout with circular directory nodes.

## Current State

### Architecture
- **UI Framework**: Dioxus 0.7.3 desktop application
- **Database**: SurrealDB 3.0.0-beta.3 with kv-surrealkv
- **Layout**: Free workspace with absolutely positioned nodes (temporary grid layout until force-directed physics)
- **Components**: Machine/Drive chips in toolbar, WorkspaceNode components for free-floating nodes

### Implemented Features
- Free workspace with SVG overlay for edges
- Machine/Drive chips in toolbar (click to open file picker)
- Circular directory nodes with child counts
- Rectangular file nodes
- Node selection (individual and lasso)
- Edge creation between nodes
- File picker functionality
- Remote machine addition
- Status indicators
- Glassmorphic UI design

### Key Architectural Changes
- Nodes are now free in the workspace (not trapped in containers)
- Machine/Drive buttons moved to toolbar as chips
- Each node tinted by its parent machine/drive's color via CSS variables
- Directory detection via child count or filesystem metadata
- Expansion state management for directory nodes

## Immediate Next Steps

### 1. Directory Expansion System
Implement the two-level expansion for directory nodes:
- **Orbit view**: Children fan out around parent in circular formation on single click
- **Enter view**: Zoom into directory showing only direct children on double-click
- **Back navigation**: Button to return to parent view
- **State management**: Track expansion state per node (collapsed/orbit/expanded)

### 2. Force-Directed Layout Engine
Replace the temporary grid layout with a physics-based system:
- Implement force-directed graph algorithm with node repulsion and edge attraction
- Add container cohesion forces to keep related nodes together
- Implement user drag pinning with persistent positions
- Store layout positions in database for persistence
- Optimize performance for large graphs

### 3. Node Grouping System
Allow users to group multiple nodes:
- Selection-based grouping (Ctrl+G or context menu)
- Group node representation (circular like directories)
- Handle edge re-routing when nodes are grouped
- Store group membership in database
- Group expansion/collapse functionality

## Technical Guidelines

### Component Architecture
- **Never nest `rsx!` macros inside `rsx!` macros** - always create subcomponents
- Use `ReadOnlySignal<T>` for read-only props in components
- Maintain clear separation between business logic and UI rendering
- Follow consistent naming conventions

### Data Management
- Use signals for reactive state management
- Implement proper error handling with Result types
- Follow immutable-first patterns where possible
- Use resources for async data loading

### UI/UX Principles
- Maintain glassmorphic design language consistently
- Ensure smooth animations for state transitions
- Provide clear visual feedback for all interactions
- Follow accessibility guidelines

## Files to Focus On

### Core Components
- `src/ui/graph.rs` - Main graph component and data loading
- `src/ui/container_components.rs` - MachineChip, WorkspaceNode, NodeHandle components
- `src/ui/graph_types.rs` - NodeView, ContainerView, and layout functions
- `assets/main.css` - Workspace and node styling

### Data Models
- `src/db.rs` - Database operations and models
- `src/models/` - Domain models (if they exist)

### Utilities
- `src/ui/file_picker.rs` - File picker implementation

## Success Criteria

### Short-term Goals
1. Directory expansion system working (orbit/enter views)
2. Force-directed layout replacing grid positioning
3. Node grouping functionality implemented
4. All features maintain the glassmorphic design aesthetic
5. Performance remains acceptable with 50-100 nodes

### Quality Standards
- No nested `rsx!` macros in the codebase
- Proper component separation maintained
- All new features properly integrated with existing functionality
- CSS variables used for theming and color management
- Smooth animations and transitions

## Known Issues to Address

1. Temporary grid layout needs replacement with force-directed physics
2. Directory expansion functionality not yet implemented
3. Node grouping system missing
4. Layout persistence not yet implemented

## Development Approach

Prioritize implementing the directory expansion system first, as this is critical for the circular node functionality. Then move to the force-directed layout engine, followed by the grouping system. Maintain the architectural principles established in the current codebase, particularly the component separation and the free workspace layout.

Focus on creating smooth, intuitive user interactions with proper visual feedback. The glassmorphic design should be preserved and enhanced throughout all new implementations.