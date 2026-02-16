# Kip Comprehensive Development Roadmap

## Current Status Overview

Kip has successfully implemented the MVP (Phase 1) as outlined in the design documents. The core functionality is working:

1. **Core Engine**: File scanning, chunked copy with blake3 hashing, resume on restart, basic error handling
2. **Location Model**: Local paths, removable drives with auto-resume
3. **UI - Mapping Graph**: Glassmorphic visual design, machine/drive containers, location nodes, drag-to-connect edges
4. **File Index**: File records with blake3 hashes, exists_at relationships
5. **Custom File Picker**: Complete implementation with column view, persistent panes, minimize/restore tabs

## Immediate Next Steps (Short-term - 2-4 weeks)

### 1. Circular Directory/Group Nodes (IN PROGRESS)
**Status**: Partially implemented, needs completion

**Components**:
- [x] NodeView data structure with is_dir, is_expanded, is_orbit, child_count
- [x] Expansion state management with Dioxus signals
- [x] Directory detection logic in load_nodes
- [x] Click handling for expansion states
- [x] Circle rendering for directories
- [x] Orbit view implementation
- [x] Expanded view implementation
- [ ] Missing CSS styles for expanded view
- [ ] Proper child count population
- [ ] Orbit view positioning refinements
- [ ] Visual styling improvements

### 2. Grouping Feature
**Status**: Not started

**Requirements** (from KIP_DESIGN_7_MAPPING_GRAPH.md):
- Select multiple nodes → group → collapse/expand
- Groups render as circles (same as directories)
- Edge merging when nodes are grouped
- NodeGroup table in SurrealDB (not yet in schema)
- Recursive grouping (groups can contain other groups)

**Implementation Plan**:
1. Add node_group table to database schema
2. Implement group creation UI (context menu or toolbar button)
3. Create GroupView data structure similar to NodeView
4. Implement group rendering (circles with member count)
5. Add group expansion logic (orbit view → enter view)
6. Implement edge merging logic
7. Add group management (rename, delete, add/remove members)

### 3. Central Output Node
**Status**: Not started

**Requirements**:
- Circular merge point at center of workspace
- Special visual treatment (slightly larger, distinct styling)
- Acts as a visual anchor for organizing the graph
- Supports both incoming and outgoing connections

### 4. Per-Node Error Badges
**Status**: Not started

**Requirements**:
- Red/yellow badges at node top-left corners
- Show count of unresolved review items
- Clicking badge scrolls review queue to relevant items

## Medium-term Goals (1-3 months)

### 5. Enhanced Graph Features
**From MVP Roadmap Phase 2**:
- Edge click/select/delete functionality
- Node context menu (delete, rename)
- Force-directed layout for larger graphs
- Layout persistence (positions saved to SurrealDB)
- Color-coded edge gradients
- Finder drag-and-drop onto graph

### 6. Remote Machine Support
**From MVP Roadmap Phase 2**:
- SSH/SFTP transfer backend
- Machine entity with connection config
- Online/offline detection
- Resume across network drops
- rsync-like delta transfer

### 7. Ninja Mode
**From MVP Roadmap Phase 2**:
- `setiopolicy_np(IOPOL_THROTTLE)` on worker threads
- 1-2 concurrent jobs max

### 8. File Preview Basics
**From MVP Roadmap Phase 2**:
- Image previews (PNG, JPG, WebP, SVG)
- Text/code syntax highlighting
- Show previews in review queue

### 9. Duplicate Detection UI
**From MVP Roadmap Phase 2**:
- UI for browsing duplicates across locations
- Suggest cleanup actions

## Long-term Vision (3-12 months)

### 10. Advanced Features (Phase 3)
- Sync intents with filesystem watching
- 3D file preview (STL, OBJ, GLTF rendering)
- Advanced previews (PDF, audio, video)
- Scheduling (cron-like per intent)
- Multi-destination intents

### 11. Platform Expansion (Phase 4)
- Linux support (inotify, udisks2)
- Windows support (ReadDirectoryChangesW, WMI)
- Cloud destinations (S3, GCS, Backblaze B2)
- CLI interface (`kip add`, `kip status`, `kip review`)

## Technical Debt and Refactoring

### 12. Code Quality Improvements
- Better separation of concerns between UI and business logic
- Improved error handling patterns
- Performance optimizations for large graphs
- Unit and integration tests
- Documentation updates

### 13. Architecture Enhancements
- Modular engine components
- Plugin system for file previews
- Extensible transfer protocols
- Better state management patterns

## Implementation Priority Matrix

### High Priority (Next 2-4 weeks)
1. Complete circular directory nodes implementation
2. Implement grouping feature
3. Add central Output node
4. Add per-node error badges

### Medium Priority (1-3 months)
5. Remote machine support
6. Ninja mode implementation
7. Enhanced graph features
8. File preview basics

### Low Priority (3+ months)
9. Advanced features and platform expansion
10. Technical debt reduction

## Success Metrics

### User Experience Metrics
- Time to complete common tasks (add location, create intent, resolve error)
- Error resolution rate without user intervention
- User satisfaction scores (when we have users)

### Technical Metrics
- Application startup time
- Memory usage during large transfers
- Database query performance
- Transfer throughput and reliability

## Risk Assessment

### High Risk Items
1. **Remote machine implementation** - Complex networking and security considerations
2. **Force-directed layout** - Performance challenges with large graphs
3. **Cross-platform support** - Significant platform-specific code required

### Medium Risk Items
1. **File preview system** - Many formats to support, potential performance issues
2. **Sync intents** - Complex change detection and conflict resolution

### Low Risk Items
1. **UI enhancements** - Mostly frontend work, lower complexity
2. **Grouping features** - Similar patterns to existing node management

## Resource Requirements

### Engineering Resources
- 1-2 engineers for core features
- 1 engineer for UI/UX improvements
- Part-time designer for visual enhancements

### Infrastructure Resources
- Test machines for different platforms
- Cloud accounts for remote testing
- Performance monitoring tools

## Milestones

### Milestone 1: Enhanced Graph (4 weeks)
- Complete circular nodes
- Implement grouping
- Add Output node
- Add error badges

### Milestone 2: Remote Support (8 weeks)
- SSH/SFTP backend
- Remote machine management
- Network resilience features

### Milestone 3: Performance & Polish (12 weeks)
- Ninja mode
- File previews
- Duplicate detection UI
- Enhanced graph features

## Dependencies

1. **SurrealDB 3.0** - Critical dependency, no alternatives considered
2. **Dioxus 0.7** - UI framework, migration path unclear
3. **Rust ecosystem** - Core language choice affects all development

## Conclusion

Kip has a solid foundation with the MVP complete. The immediate focus should be on enhancing the graph UI with circular nodes, grouping, and related features. This will provide significant user value while building toward the more complex remote and synchronization features. The roadmap balances user-facing improvements with technical foundations, ensuring steady progress toward the long-term vision.