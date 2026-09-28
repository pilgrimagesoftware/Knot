# Tasks

## 1. Store: gap-based move that carries companions

- [ ] 1.1 Replace `AgentStore::reorder` in `crates/knot-agents/src/store/ordering.rs` with `move_agent_to_gap(workspace_id, id, gap) -> bool` per design.md, with a doc comment on the gap model and the group rule; verify `cargo build -p knot-agents` succeeds and no caller of `reorder` remains (`grep -rn "\.reorder(" crates`)
- [ ] 1.2 Port `reorder_and_move_to_workspace_update_membership` to the new function and add store tests for each `agent-lifecycle` scenario: owner moves with its companion, a gap inside a companion group is refused, the gaps either side of the group return `false`, a companion id is refused, gap `len` moves to the end; verify `cargo test -p knot-agents` passes

## 2. Sidebar drag mechanics

- [ ] 2.1 Add `crates/knot/src/workspace_window/sidebar_drag.rs` with `AgentDrag`, `DropEdge`, and the pure `drop_gap` and `drop_line_edge` functions, declared from `workspace_window/mod.rs`; verify unit tests cover upper and lower halves of standalone, owner and companion rows, the end of the list, and `None` within the dragged group's span
- [ ] 2.2 Add `AgentRowDrag` state to `WorkspaceWindow`, plus `agent_drag_moved` and `agent_dropped`, which call `move_agent_to_gap` and, only on `true`, `persist_agents` and `cx.notify()`; verify `make build` succeeds
- [ ] 2.3 Add the insertion line element and `AgentDragPreview` (debug selectors `agent-drop-line`, `agent-drag-preview`), placed from captured row bounds and kept inside the window; verify `make lint` passes

## 3. Full sidebar wiring

- [ ] 3.1 In `render/sidebar.rs`, move the agent rows into their own container that captures bounds with `on_children_prepainted`. Give non-companion rows `on_drag` and every agent row `on_drag_move`/`on_drop`, and let the empty space below the rows accept a drop at the end; verify `make size-check` passes and the existing sidebar tests still pass
- [ ] 3.2 Add window tests in `crates/knot/src/tests/workspace_window_agent_drag.rs`, driven like `workspace_manager_drag.rs`, for every `agent-list-ui` scenario on the full sidebar: below, above, to the end, click is not a drag, companion row does not drag, drop onto a companion, pinned rows stay first, line placement, no line on own place, preview follows the pointer, a completed drag leaves the selection unchanged; verify `cargo test -p knot workspace_window_agent_drag` passes
- [ ] 3.3 Add a test that a completed reorder persists `agent_ids` through a store root from `with_store_root` and that a no-op drag leaves the workspaces file untouched (never `Settings::default()`); verify it passes

## 4. Compact sidebar wiring

- [ ] 4.1 Wire the same drag handlers into `render/sidebar_compact.rs`'s avatar rows; verify with a window test that drags in a sidebar narrower than 160px and asserts the new order

## 5. Integration

- [ ] 5.1 Run `make` (fmt-check, size-check, lint, test, build); verify it passes
- [ ] 5.2 Launch the app with a throwaway store holding a workspace with a companion, drag rows in the full and compact sidebar, relaunch, and confirm the order survived; verify by recording the result in the PR description
