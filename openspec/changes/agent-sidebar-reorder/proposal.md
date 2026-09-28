# Proposal

## Why

A workspace window's sidebar lists its agents in the order they were
created, and nothing in the UI changes that order. The only way to put an
agent somewhere else is to move it to another workspace and back, which puts
it at the end. The Swift app lets the user drag an agent row to a new place
in the list, and the store already has an unwired reorder for it.
`agent-lifecycle` promises that an agent can be reordered within its
workspace, but nothing in the Rust UI does it.

## What Changes

- An agent row in the workspace sidebar can be dragged up or down the agent
  list and dropped into a new place. The new order is saved at once and is
  still there when the window reopens.
- While a row is dragged, a copy of the row follows the pointer and an
  insertion line shows where the row will land, matching the Workspace
  Manager's workspace drag.
- An agent moves with its companions. A companion row is not dragged on its
  own, and no agent can be dropped between an owner and its companions.
- The Dashboard and Pull Requests rows stay pinned above the agent list. They
  are not dragged, and nothing is dropped above them.
- Dragging works in the compact sidebar too, where a row is only its avatar.
- A click without a drag still selects the agent, and right-clicking still
  opens the row's context menu.

## Non-Goals

- Dragging an agent into another workspace or another window. Move to
  Workspace on the context menu covers that.
- Reordering companions within their owner's group.
- Keyboard or menu commands for reordering, and an MCP tool that reorders
  agents.
- Writing a `workspace-manager-ui` requirement for the workspace drag that
  already exists. This change copies its code pattern, not its spec.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-list-ui`: adds requirements for dragging an agent row to a new
  place, the drag's visual feedback, which rows can be dragged and where they
  can be dropped, and saving the new order.
- `agent-lifecycle`: "Ordering and workspace placement" gains the rule that
  reordering an agent moves its companions with it, so each owner's
  companions stay directly after it.

## Impact

- `crates/knot-agents/src/store/ordering.rs`: replaces the unwired
  index-based `AgentStore::reorder` with a gap-based move that carries an
  agent's companions, modeled on `move_workspace_to_gap`.
- `crates/knot/src/workspace_window/`: a new sidebar drag module (drag
  payload, drop target, insertion line, preview) and drag state on
  `WorkspaceWindow`. `render/sidebar.rs` and `render/sidebar_compact.rs` wire
  each row to it. The new order is saved through
  `WorkspaceWindow::persist_agents`.
- The saved format does not change. `Workspace.agent_ids` already holds the
  order.
- Tests: store unit tests for the move, and window-driven drag tests
  alongside `crates/knot/src/tests/workspace_manager_drag.rs`.
