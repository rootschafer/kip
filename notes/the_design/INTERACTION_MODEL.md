# Kip Interaction Model

**Date:** February 22, 2026  
**Status:** Design Specification

---

## Overview

Kip uses a **select-then-act** interaction model with context menus for node operations. This provides a consistent, extensible interface that scales well as features are added.

---

## Core Interactions

### 1. Single Click — Select

**Action:** Click once on a node

**Result:** 
- Node becomes selected (highlighted with border glow)
- Previous selection is cleared (unless modifier held)
- Enables keyboard shortcuts and toolbar actions for the selected node

**Modifier Keys:**
- **Shift+Click:** Toggle selection (add/remove from selection)
- **Ctrl+Click:** Add to selection (don't clear existing)

---

### 2. Click + Drag — Move Node

**Action:** Click and hold, then drag

**Result:**
- Node follows cursor
- Release to drop at new position
- Position is saved to database

**Multi-Select Drag:**
- If multiple nodes are selected, dragging any one of them moves all selected nodes together
- Maintains relative positions between selected nodes

---

### 3. Double Click — Context Menu

**Action:** Rapid double-click on a node

**Result:** Context menu appears with node-type-specific actions

---

## Context Menus by Node Type

### Machine/Drive Nodes

```
┌─────────────────────────────┐
│  Into (ENTER)               │
│  Expand (SPACE)             │
│  ─────────────────────────  │
│  Tag                        │
│  Move (M)                   │
│  Copy                       │
│  Copy Path                  │
│  Open in Finder             │
│  ─────────────────────────  │
│  Delete                     │
└─────────────────────────────┘
```

**Actions:**

| Action | Shortcut | Description |
|--------|----------|-------------|
| **Into** | `ENTER` | Navigate into the machine/drive, showing its contents in the workspace |
| **Expand** | `SPACE` | Toggle orbit view — children fan out around the node in a circular arrangement |
| **Tag** | — | Add/remove color tags for organization |
| **Move** | `M` | Enter move mode — click destination to move all contents |
| **Copy** | — | Copy node reference to clipboard |
| **Copy Path** | — | Copy full filesystem path to clipboard |
| **Open in Finder** | — | Open the location in macOS Finder |
| **Delete** | `DELETE` | Remove the node (prompts for confirmation) |

---

### Directory Nodes

```
┌─────────────────────────────┐
│  Into (ENTER)               │
│  Expand (SPACE)             │
│  ─────────────────────────  │
│  Tag                        │
│  Move (M)                   │
│  Copy                       │
│  Copy Path                  │
│  Open in Finder             │
│  ─────────────────────────  │
│  Delete                     │
└─────────────────────────────┘
```

Same as Machine/Drive, but:
- "Into" navigates into the directory
- "Expand" shows immediate children in orbit view

---

### File Nodes

```
┌─────────────────────────────┐
│  Open (ENTER)               │
│  ─────────────────────────  │
│  Tag                        │
│  Move (M)                   │
│  Copy                       │
│  Copy Path                  │
│  Open in Finder             │
│  Open With...               │
│  ─────────────────────────  │
│  Delete                     │
└─────────────────────────────┘
```

**Actions:**

| Action | Shortcut | Description |
|--------|----------|-------------|
| **Open** | `ENTER` | Open file in default application |
| **Open With...** | — | Choose application to open file |
| **Tag** | — | Add/remove color tags |
| **Move** | `M` | Enter move mode |
| **Copy** | — | Copy file reference |
| **Copy Path** | — | Copy full filesystem path |
| **Open in Finder** | — | Reveal in Finder |
| **Delete** | `DELETE` | Delete file (with confirmation) |

---

## Keyboard Shortcuts

### Global

| Key | Action |
|-----|--------|
| `ESC` | Deselect all, close menus |
| `DELETE` | Delete selected node(s) |
| `SPACE` | Expand selected node (orbit view) |
| `ENTER` | Primary action (Into/Open depending on node type) |
| `M` | Move mode |
| `CMD+A` | Select all nodes in current view |
| `CMD+F` | Find/search nodes |

### Selection

| Key | Action |
|-----|--------|
| `Shift+Click` | Toggle selection |
| `Ctrl+Click` | Add to selection |
| `Drag` (empty space) | Lasso selection |

### Navigation

| Key | Action |
|-----|--------|
| `ENTER` (on Machine/Drive/Dir) | Navigate into |
| `BACKSPACE` | Navigate to parent |
| `CMD+Up` | Navigate to parent |
| `CMD+Down` | Navigate into selected |

---

## Edge Creation

**Current Method:** Drag from node's edge handle to another node

**Future Method (TBD):**
- Context menu option: "Connect to..."
- Toolbar button: "Create sync"
- Keyboard: `CMD+L` to link selected nodes

---

## Lasso Selection

**Action:** Click and drag on empty workspace area

**Result:**
- Dashed rectangle appears
- On release, all nodes whose center falls within rectangle are selected
- Adds to existing selection (doesn't replace)

**Modifier:**
- **Shift+Drag:** Same as above (explicit lasso mode)

---

## Design Principles

1. **Consistent:** Same interaction patterns across all node types
2. **Discoverable:** Context menus show available actions
3. **Extensible:** New actions can be added to menus without UI changes
4. **Keyboard-friendly:** All actions have keyboard shortcuts
5. **Non-destructive:** Destructive actions require confirmation

---

## Future Enhancements

### Right-Click Context Menu
- Currently double-click opens context menu
- Future: Right-click opens context menu, double-click has special behavior

### Touch/Trackpad Gestures
- Pinch to zoom
- Two-finger swipe to pan
- Long-press for context menu

### Quick Actions Toolbar
- Floating toolbar appears near selection
- Shows most common actions for selected node type
