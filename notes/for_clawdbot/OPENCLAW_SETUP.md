# OpenClaw Setup for Kip Development

**Date:** February 22, 2026

---

## Quick Start

### 1. Clone the Repository

```bash
# Clone Kip repository
git clone https://github.com/yourusername/kip.git
cd kip

# Install Rust toolchain (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install Dioxus CLI
cargo install --git https://github.com/DioxusLabs/dioxus dioxus-cli

# Verify setup
dx --version
cargo --version
```

### 2. Install OpenClaw

```bash
# Install OpenClaw (multi-agent coordination system)
pip install openclaw

# Or from source
git clone https://github.com/openclaw/openclaw.git
cd openclaw
pip install -e .
```

### 3. Configure OpenClaw

Create `openclaw.yaml` in the project root:

```yaml
project:
  name: kip
  root: .
  
agents:
  - name: architect
    role: design_and_planning
    model: claude-sonnet-4-20250514
    context_files:
      - notes/the_design/INTERACTION_MODEL.md
      - notes/the_design/COMPREHENSIVE_DEVELOPMENT_PLAN.md
      - notes/the_design/START_HERE.md
      
  - name: frontend_dev
    role: ui_implementation
    model: claude-sonnet-4-20250514
    context_files:
      - src/ui/graph.rs
      - src/ui/graph_nodes.rs
      - src/ui/graph_store.rs
      - assets/main.css
      
  - name: backend_dev
    role: backend_implementation
    model: claude-sonnet-4-20250514
    context_files:
      - src/api/
      - src/engine/
      - src/db/
      
  - name: tester
    role: testing_and_validation
    model: claude-sonnet-4-20250514
    context_files:
      - tests/
      - notes/the_design/INTERACTION_MODEL.md

tasks:
  # Phase 1: Interaction Model
  - id: interaction_refactor
    name: "Fix Click/Drag Conflict"
    description: |
      Implement new interaction model:
      - Single click selects only (no drag start)
      - Click + drag moves node
      - Double click opens context menu
    assigned_to: frontend_dev
    dependencies: []
    files:
      - src/ui/graph.rs
      - src/ui/graph_nodes.rs
      - src/ui/graph_store.rs
    acceptance_criteria:
      - "Single click only selects node"
      - "Drag moves node without re-selecting"
      - "Double click opens context menu"
      
  - id: context_menus
    name: "Implement Context Menus"
    description: |
      Add node-type-specific context menus:
      - Machine/Drive: Into, Expand, Tag, Move, Copy, Copy Path, Open in Finder, Delete
      - Directory: Into, Expand, Tag, Move, Copy, Copy Path, Open in Finder, Delete
      - File: Open, Tag, Move, Copy, Copy Path, Open in Finder, Open With, Delete
    assigned_to: frontend_dev
    dependencies: [interaction_refactor]
    files:
      - src/ui/graph.rs
      - src/ui/graph_store.rs
      - assets/main.css
      
  - id: keyboard_shortcuts
    name: "Add Keyboard Shortcuts"
    description: |
      Implement keyboard shortcuts:
      - ENTER: Primary action (Into/Open)
      - SPACE: Expand (orbit view)
      - DELETE: Delete selected
      - ESC: Deselect all
      - CMD+A: Select all
      - M: Move mode
    assigned_to: frontend_dev
    dependencies: [interaction_refactor]
    files:
      - src/ui/graph.rs
      
  - id: test_interactions
    name: "Test Interaction Model"
    description: |
      Validate all interaction changes:
      - Click behavior works correctly
      - Context menus appear and function
      - Keyboard shortcuts work
      - No regressions in existing functionality
    assigned_to: tester
    dependencies: [interaction_refactor, context_menus, keyboard_shortcuts]
    files:
      - tests/

workflow:
  # Sequential phases
  - phase: interaction_model
    tasks: [interaction_refactor, context_menus, keyboard_shortcuts]
    
  - phase: validation
    tasks: [test_interactions]
    
coordination:
  # How agents communicate
  sync_points:
    - after: interaction_refactor
      review_by: [architect, tester]
    - after: context_menus
      review_by: [architect]
    - after: keyboard_shortcuts
      review_by: [architect, tester]
      
  # Code review requirements
  code_review:
    required: true
    reviewers: [architect]
    min_approvals: 1
    
  # Build/test requirements
  ci_checks:
    - dx check
    - cargo test --test api_tests
    - cargo test --test integration_tests
```

---

## Running OpenClaw

### Start the Coordination System

```bash
cd kip
openclaw start --config openclaw.yaml
```

### Monitor Progress

```bash
# View agent status
openclaw status

# View task progress
openclaw tasks

# View logs for specific agent
openclaw logs --agent frontend_dev

# View all logs
openclaw logs --all
```

### Agent Commands

```bash
# Assign task to agent
openclaw assign --agent frontend_dev --task context_menus

# Get agent status
openclaw agent --name frontend_dev

# Request agent to review code
openclaw review --agent architect --pr 123

# Pause/resume agent
openclaw pause --agent frontend_dev
openclaw resume --agent frontend_dev
```

---

## Task Breakdown by Agent

### Architect Agent

**Responsibilities:**
- Review design decisions
- Ensure consistency with `INTERACTION_MODEL.md`
- Code review for all changes
- Update documentation as needed

**Tasks:**
1. Review interaction refactor implementation
2. Review context menu implementation
3. Review keyboard shortcut implementation
4. Update `COMPREHENSIVE_DEVELOPMENT_PLAN.md` with progress

**Commands:**
```bash
openclaw task architect --action review --pr <PR_NUMBER>
openclaw task architect --action update-docs --file notes/the_design/IMPLEMENTATION_SUMMARY.md
```

---

### Frontend Dev Agent

**Responsibilities:**
- Implement UI changes in `src/ui/`
- Update CSS in `assets/main.css`
- Ensure Dioxus best practices
- Test UI changes locally

**Tasks:**
1. Fix click/drag conflict in `graph.rs`
2. Implement context menu system
3. Add keyboard event handlers
4. Add selection highlighting CSS
5. Add context menu CSS

**Commands:**
```bash
openclaw task frontend_dev --action implement --task interaction_refactor
openclaw task frontend_dev --action test --command "dx serve --platform desktop"
openclaw task frontend_dev --action format --command "dx fmt && cargo fmt"
```

---

### Backend Dev Agent

**Responsibilities:**
- Maintain API layer consistency
- Ensure database queries work correctly
- Fix any SurrealDB type issues
- Optimize performance

**Tasks:**
1. Review any API changes needed for new interactions
2. Fix SurrealDB type coercion issues in tests
3. Optimize node selection queries
4. Ensure context menu data is available via API

**Commands:**
```bash
openclaw task backend_dev --action review --files src/api/
openclaw task backend_dev --action fix --issue "SurrealDB type coercion"
openclaw task backend_dev --action test --command "cargo test --test api_tests"
```

---

### Tester Agent

**Responsibilities:**
- Write integration tests
- Validate acceptance criteria
- Report bugs and regressions
- Ensure test coverage

**Tasks:**
1. Write tests for click behavior
2. Write tests for context menus
3. Write tests for keyboard shortcuts
4. Run full test suite
5. Report any regressions

**Commands:**
```bash
openclaw task tester --action write-test --feature "click behavior"
openclaw task tester --action validate --task interaction_refactor
openclaw task tester --action run-tests --suite "all"
```

---

## Build & Test Commands

### Desktop App

```bash
# Check (fast)
dx check

# Build
dx build

# Run with hot reload
dx serve --platform desktop

# Format
dx fmt
```

### CLI

```bash
# Build CLI
cargo build --bin kip-cli

# Run CLI
./target/debug/kip-cli --help

# Test CLI
cargo test --bin kip-cli
```

### Tests

```bash
# Unit tests
cargo test --test api_tests -- --test-threads=1

# Integration tests
cargo test --test integration_tests -- --test-threads=1

# All tests
cargo test

# Format code
cargo fmt
```

---

## Coordination Workflow

### Daily Standup

```bash
# Each agent reports status
openclaw standup

# Output example:
# architect: Reviewed PR #45, approved
# frontend_dev: Implemented context menus, ready for review
# backend_dev: Fixed 2 SurrealDB type issues
# tester: Wrote 5 tests for click behavior, found 1 bug
```

### Code Review Flow

1. **Frontend dev** implements feature
2. **Frontend dev** requests review: `openclaw review --request --agent architect`
3. **Architect** reviews: `openclaw review --approve --pr 123` or `openclaw review --request-changes --pr 123 --comment "..."`
4. **Frontend dev** addresses feedback
5. **Tester** validates: `openclaw validate --pr 123`
6. **Merge** when all approvals received

### Conflict Resolution

If agents disagree:

```bash
# Escalate to human
openclaw escalate --issue "Agents disagree on implementation approach" --agents frontend_dev,architect

# Request design clarification
openclaw clarify --from frontend_dev --to architect --question "Should context menu close on outside click?"
```

---

## Monitoring & Metrics

### Agent Productivity

```bash
# View agent metrics
openclaw metrics --agent frontend_dev

# Output:
# Tasks completed: 3
# Code reviews: 2
# Tests written: 5
# Build failures: 0
```

### Project Progress

```bash
# View project dashboard
openclaw dashboard

# Output:
# Phase: Interaction Model (60% complete)
# Tasks: 3/5 complete
# Tests: 12/20 passing
# Build: ✅ Passing
```

---

## Troubleshooting

### Agent Stuck

```bash
# Check agent logs
openclaw logs --agent frontend_dev --tail 100

# Restart agent
openclaw restart --agent frontend_dev

# Reassign task
openclaw reassign --task interaction_refactor --from frontend_dev --to backend_dev
```

### Build Failures

```bash
# Get build error summary
openclaw build-errors

# Assign fix to appropriate agent
openclaw assign --agent backend_dev --task fix-build-errors
```

### Test Failures

```bash
# Get test failure summary
openclaw test-failures

# Assign fix to appropriate agent
openclaw assign --agent frontend_dev --task fix-test-failures
```

---

## Best Practices

### For Agents

1. **Read documentation first** — Always read relevant docs in `notes/the_design/`
2. **Test locally** — Run `dx check` and `cargo test` before committing
3. **Small PRs** — Keep changes focused and reviewable
4. **Update docs** — If behavior changes, update documentation
5. **Communicate** — Use `openclaw clarify` when unsure

### For Humans

1. **Review escalations promptly** — Don't block agents
2. **Clarify requirements** — Update docs if agents are confused
3. **Monitor progress** — Check `openclaw dashboard` daily
4. **Intervene when needed** — Some decisions need human judgment

---

## Example Session

```bash
# Start OpenClaw
$ openclaw start --config openclaw.yaml
OpenClaw started with 4 agents

# Check status
$ openclaw status
architect:    idle
frontend_dev: working on interaction_refactor (60%)
backend_dev:  idle
tester:       idle

# Frontend dev completes task
$ openclaw task frontend_dev --complete interaction_refactor
Task interaction_refactor marked complete

# Request review
$ openclaw review --request --agent architect --task interaction_refactor
Review requested from architect

# Architect reviews
$ openclaw review --approve --task interaction_refactor
Task interaction_refactor approved

# Start next task
$ openclaw assign --agent frontend_dev --task context_menus
Task context_menus assigned to frontend_dev

# Check progress
$ openclaw dashboard
Phase: Interaction Model (40% complete)
Tasks: 1/4 complete, 1 in progress
Build: ✅ Passing
Tests: ✅ 12/12 passing
```

---

## Next Steps

1. **Clone repository** (see Quick Start above)
2. **Install OpenClaw** (see Quick Start above)
3. **Create `openclaw.yaml`** (copy from above)
4. **Start coordination** — `openclaw start --config openclaw.yaml`
5. **Monitor progress** — `openclaw dashboard`

For questions, see:
- `notes/the_design/START_HERE.md` — Project overview
- `notes/the_design/INTERACTION_MODEL.md` — Interaction specification
- `notes/the_design/COMPREHENSIVE_DEVELOPMENT_PLAN.md` — Roadmap

