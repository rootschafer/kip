# Kip Development: Getting Started

## Critical Protocol: Design Documentation Accuracy

**DO NOT mark features as complete unless you have personally verified they work in the running application.** 

Design documentation in `./notes/the_design/` serves as the authoritative reference for:
- Current implementation status
- Feature completeness for handoffs between AI agents
- Planning for future development phases

**False completion markers cause cascading issues:**
- Future agents waste time implementing already-completed features
- Agents break working code trying to "fix" non-existent issues
- Development velocity decreases due to miscommunication

**Verification protocol:**
1. Run the application with `dx serve --platform desktop`
2. Manually test the feature in the UI
3. Confirm it behaves as specified in the design docs
4. Only then update the completion status

## Directory Overview

### Core Architecture
- `KIP_DESIGN_1.md` — High-level vision and architecture overview
- `KIP_DESIGN_2_DATA_MODEL.md` — Database schema and data relationships
- `KIP_DESIGN_3_INTENT_LIFECYCLE.md` — Transfer intent states and progression
- `KIP_DESIGN_4_ARCHITECTURE.md` — Technical architecture and component relationships
- `KIP_DESIGN_5_ERROR_HANDLING.md` — Error classification and user review system

### UI Components
- `KIP_DESIGN_6_MVP.md` — Minimum viable product features and scope
- `KIP_DESIGN_7_MAPPING_GRAPH.md` — Main graph UI, nodes, edges, and interactions
- `KIP_DESIGN_8_FILE_PICKER.md` — Custom file picker with column navigation

### Implementation Guides
- `CIRCULAR_NODES_IMPLEMENTATION.md` — Technical guide for directory circle nodes
- `FILE_PICKER_IMPLEMENTATION.md` — File picker component implementation details
- `COMPREHENSIVE_DEVELOPMENT_PLAN.md` — Overall roadmap from prototype to production
- `IMPLEMENTATION_SUMMARY.md` — Brief summary of current implementation status

### Agent Coordination
- `AGENTS.md` — Technical reference for Dioxus/SurrealDB patterns and gotchas
- `START_HERE.md` — This file, explaining design doc protocols

## Updating Design Documents

### Task Status System
Features/tasks use a four-state system:

1. **[ ]** (empty box) - NOT_STARTED: Feature hasn't been implemented
2. **[~]** (tilde) - PARTIALLY_DONE: Feature has some implementation but is incomplete
3. **[√]** (checkmark) - AI_THINKS_DONE: AI believes implementation is complete but hasn't been validated by user
4. **[x]** (x) - VALIDATED_COMPLETE: User has verified the feature works as specified

### When to Update
- **After implementing** a feature (mark as [√] AI_THINKS_DONE)
- **After verifying** functionality works end-to-end (upgrade to [x] VALIDATED_COMPLETE)
- **When discovering existing implementation** that wasn't properly documented
- **When design specifications change** based on implementation insights

### How to Update
1. **For partial completion**: Use [~] and add "(PARTIAL)" to description
2. **For believed completion**: Use [√] and add "(AI CONFIRMED)" to description
3. **For user-validated completion**: Use [x] and add "(USER VERIFIED)" to description
4. **For unimplemented**: Keep as [ ] 
5. **Keep descriptions factual**, not aspirational
6. **Note any deviations** from original design with brief explanations

### Verification Checklist
Before upgrading from [√] to [x], verify:
- [ ] Feature works in the running application
- [ ] Matches the specified behavior in design docs
- [ ] Doesn't break existing functionality
- [ ] Handles edge cases reasonably
- [ ] Performance is acceptable

## Handoff Protocol

When ending a development session:
1. Update design docs to reflect actual implementation status
2. Document any design changes or discoveries
3. Note any issues or gotchas for the next agent
4. Ensure all changes compile and basic functionality works

## Critical Current Issues

When working on the directory expansion functionality, pay special attention to these known issues:
1. **SVG Coordinate System Alignment**: Mouse coordinates don't align with SVG overlay coordinates, causing cursor offset in edge creation
2. **Click vs Drag Detection**: Current implementation interferes with edge creation when trying to expand nodes
3. **Orbit View Implementation**: Children not properly fanned out around parent nodes in orbit state
4. **Enter View Implementation**: Workspace not properly filtered to show only direct children of entered directory

## Key Implementation Notes

- The PickerManager is now implemented as a Store for reactive state management
- Directory nodes have dynamic sizing based on total descendant count
- Expansion state is tracked as (is_orbit, is_expanded) tuples in a HashMap
- Use the async move { ... } pattern for event handlers that need async functionality
- The workspace uses absolute positioning with temporary grid layout (to be replaced with force-directed)

Remember: The design docs are the single source of truth for project status. Accuracy is paramount for efficient development.