# Design

## Context

- `WorkspaceWindow::frame_snapshot` (`render/mod.rs`) maps
  `Workspace.agent_ids` to `AgentRow`s in order, and `agent_rows`
  (`render/sidebar.rs`) renders them one to one. Companions are separate
  rows, indented, placed after their owner. The Dashboard and Pull Requests
  rows come before them and are not in `agent_ids`.
- `AgentStore::reorder(workspace_id, from, to)` (`knot-agents`,
  `store/ordering.rs`) is an index-based `remove`/`insert` with no caller
  outside a store test. It knows nothing about companions.
- The Workspace Manager's drag (`workspace_manager/drag.rs`, `state.rs`,
  `render.rs`) is the pattern to copy. It uses a `WorkspaceDrag { id, row }`
  payload, pure `drop_gap`/`drop_line_edge` functions, a `DropTarget { row,
  gap }` updated from `on_drag_move`, row bounds captured with
  `on_children_prepainted`, a `WorkspaceDragPreview` placed from those bounds,
  and `AgentStore::move_workspace_to_gap`. Its tests are in
  `crates/knot/src/tests/workspace_manager_drag.rs`.

## Goals / Non-Goals

**Goals:**

- Reuse the Workspace Manager's gap model and preview, so both lists drag the
  same way and share test techniques.
- Keep the companion rule in the store, where a unit test can check it
  without a window.

**Non-Goals:**

- A shared, generic reorderable-list component for both lists. Two users do
  not justify the abstraction yet. See Decisions.

## Decisions

### Gap-based store move that carries companions

Replace `AgentStore::reorder` with `move_agent_to_gap(workspace_id, id, gap)
-> bool` in `store/ordering.rs`. `gap` counts places in `agent_ids`, as
`move_workspace_to_gap` does: gap `n` is above the `n`th id, gap `len` is
after the last.

The moved unit is a group: the agent plus the ids that directly follow it and
are its companions (`created_by == Some(id) && is_companion`). The move
returns `false` and changes nothing when:

- `id` is a companion, or not in the workspace;
- `gap` falls inside any group, i.e. directly before a companion;
- `gap` is within the group's own span, from its first index to one past its
  last, which leaves the order unchanged.

Otherwise the group is removed and reinserted at `gap`, adjusted for the ids
removed above it.

Alternative: keep index-based `reorder` and let the UI compute indices.
Rejected. The end of the list is not an index, and the companion rule would
live in UI code that only window tests reach. `reorder`'s one test moves to
the new function. No other caller exists, so nothing else changes.

### Row-to-gap mapping in the window, snapped to groups

A pure `drop_gap(rows, row, edge, dragged) -> Option<usize>` in the new drag
module maps the hovered row and pointer half to a gap:

- An owner or standalone row: upper half is the gap above it. Lower half is
  the gap after its whole group, not directly below the row.
- A companion row: the gap above or below its owner's group, whichever the
  pointer is nearer, from the pointer's position within the group's combined
  bounds.
- `None` for gaps inside the dragged group's own span, so no line is drawn
  and the drop is a no-op.

`rows` is a slice of per-row flags (`is_companion`, owner index) taken from
the frame snapshot, so the function has no store or window dependency and
is unit-tested directly, as `drop_gap` is in the Workspace Manager.
`drop_line_edge` is reused in shape: one gap draws one line, on the bottom
edge of the row above it, or on the top edge of the first row for gap 0.

### Where the drag lives

- New `crates/knot/src/workspace_window/sidebar_drag.rs`: the `AgentDrag {
  id, row }` payload, `drop_gap`, `drop_line_edge`, the insertion line
  element, `AgentDragPreview`, and `WorkspaceWindow::agent_drag_moved` /
  `agent_dropped`.
- `WorkspaceWindow` gains an `agent_drag: AgentRowDrag { target:
  Option<DropTarget>, row_bounds: Rc<RefCell<Vec<Bounds<Pixels>>>> }`.
- `render/sidebar.rs` and `render/sidebar_compact.rs` put the agent rows in
  their own container, without the pinned rows. That container captures
  bounds with `on_children_prepainted`, so row indices match `agent_ids`
  indices and pinned rows can never be drop places. Non-companion rows get
  `on_drag`. Every agent row gets `on_drag_move` and `on_drop`. The
  container's empty space below the rows accepts a drop as gap `len`.
- `agent_dropped` calls `move_agent_to_gap`. Only when it returns `true`
  does it call `WorkspaceWindow::persist_agents` and `cx.notify()`.

Alternative: extract a generic reorderable list from the Workspace Manager
and use it in both places. Deferred. The two lists differ in grouping,
pinned rows and compact layout, and a shared version would be designed
around two callers only.

### Whole row as the drag source

The row keeps its current look, per `knot-ui-conventions.md`'s agent row
shape, and has no handle. gpui starts a drag only after the pointer moves
past its threshold, so `on_click` still fires for a press without movement.
The Workspace Manager uses a handle because its rows hold text fields and
buttons. Agent rows hold neither.

## Risks / Trade-offs

- [A drag also fires `on_click` on release and selects the agent] → Check in
  a window test that a completed drag does not change the selection. If gpui
  does fire the click, check the drag state in the click handler.
- [Row bounds are captured one frame late] → Same as the Workspace Manager.
  The bounds are only read during a drag, after at least one prepaint.
- [An agent added or removed mid-drag shifts indices] → `agent_dropped`
  resolves the dragged agent by id, and `move_agent_to_gap` re-checks group
  boundaries against the current order. A stale gap either lands on a valid
  place or is refused. It never splits a group.
- [`move_agent_to_gap` assumes companions directly follow their owner] →
  That is `agent-lifecycle`'s placement rule. If an order breaks it, the
  group is only the companions that are directly after the owner, and the
  move still cannot split what is contiguous.
