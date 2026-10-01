# Proposal

## Why

The Workspaces window lists every workspace in a plain column with no scroll
container (`crates/knot/src/workspace_manager/render.rs:216` renders the rows
as `v_flex().gap_2().children(rows)` directly inside the padded content
area). Once the rows are taller than the window, the overflow is clipped and
the user has no way to reach the workspaces below the fold - no scrollbar, no
scroll wheel handling. The Swift reference scrolls: its workspace list is
wrapped in a `ScrollView` (`Skwad/Views/Workspace/WorkspaceBarView.swift:34`),
and the port's own agent sidebar and processes pane both scroll. The workspace
list is the one place in the manager window the intent to scroll was lost.

## What Changes

- **The workspace list scrolls.** Rows still render in one column, but the
  container that holds them becomes vertically scrollable and is bounded by
  the window's content area, so every workspace is reachable by scroll wheel
  or scrollbar regardless of how many exist.
- **The chrome does not scroll.** The title bar, toolbar (command center and
  New Workspace) and any error banner stay pinned; only the row list scrolls
  beneath them.
- **The window keeps its current shape.** One-column list, double-click to
  open, drag to reorder, rename/delete buttons per row - unchanged. This
  change touches only how the list grows overflow, not what a row does.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-manager-ui`: adds a requirement that the workspace list scrolls
  when it outgrows the window, which the spec currently does not cover at all.

## Impact

- `crates/knot/src/workspace_manager/render.rs` - the row container becomes a
  bounded, vertically scrollable element; the toolbar/error chrome stays
  outside it.
- `openspec/specs/workspace-manager-ui/spec.md` - new delta requirement with
  scenarios for few and many workspaces.
- No new dependencies. The scroll pattern already used by the processes pane
  (`overflow_y_scroll` + a max height on a `div`,
  `crates/knot/src/workspace_window/render/processes_pane.rs:295`) applies
  here unchanged.
- No API, localization or data-model changes. The window's state and actions
  are untouched; this is purely a render change.