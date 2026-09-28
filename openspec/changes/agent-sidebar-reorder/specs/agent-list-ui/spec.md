# Spec Delta

## ADDED Requirements

### Requirement: An agent row can be dragged to a new place in the sidebar

The user SHALL be able to press on an agent row in the workspace sidebar,
drag it up or down the agent list, and release it to move that agent to the
place it was released over. The move SHALL follow `agent-lifecycle`'s
ordering requirement: the agent's companions move with it. The whole row
SHALL start the drag. The row SHALL NOT gain a separate drag handle.

A press that is released before the pointer moves far enough to start a drag
SHALL remain a click and select the agent, as it does today. Right-clicking
a row SHALL still open its context menu.

The drop place SHALL be decided by the half of the row under the pointer:
the upper half places the agent above that row, the lower half below it.
Releasing below the last agent row SHALL move the agent to the end of the
list.

Dragging SHALL work the same way in the full sidebar and in the compact
sidebar.

This matches the Swift app, which drags the row to a place above or below
another agent. The Swift app draws companions as chips inside their owner's
row. The Rust sidebar draws them as their own indented rows, so the rules
for which rows can be dragged are stated here (see the next requirement).

#### Scenario: Dragging an agent below another

- **WHEN** the sidebar lists `A`, `B`, `C` and the user drags `A` and
  releases it over the lower half of `B`
- **THEN** the sidebar lists `B`, `A`, `C`

#### Scenario: Dragging an agent above another

- **WHEN** the sidebar lists `A`, `B`, `C` and the user drags `C` and
  releases it over the upper half of `A`
- **THEN** the sidebar lists `C`, `A`, `B`

#### Scenario: Dragging an agent to the end

- **WHEN** the sidebar lists `A`, `B`, `C` and the user drags `A` and
  releases it below `C`
- **THEN** the sidebar lists `B`, `C`, `A`

#### Scenario: A click is not a drag

- **WHEN** the user presses on `B`'s row and releases without moving the
  pointer
- **THEN** `B` is selected and the order is unchanged

#### Scenario: Dragging in the compact sidebar

- **WHEN** the sidebar is compact, lists `A`, `B`, `C`, and the user drags
  `A`'s avatar and releases it over the lower half of `B`'s avatar
- **THEN** the sidebar lists `B`, `A`, `C`

### Requirement: Companions and pinned rows are not reordered on their own

A companion row SHALL NOT start a drag. Its owner's row does, and the
companions move with it. While a row is dragged, the places between an
owner and its companions, and between two companions of one owner, SHALL NOT
be drop places. Releasing over a companion row SHALL place the dragged agent
above or below that companion's whole owner group, whichever is nearer.

The Dashboard row and the Pull Requests row SHALL NOT start a drag, and SHALL
NOT be drop places. They stay pinned above the agent list.

#### Scenario: A companion row does not drag

- **WHEN** the user presses on a companion's row and moves the pointer
- **THEN** no drag starts and the order is unchanged

#### Scenario: Dropping onto a companion places the agent outside the group

- **WHEN** the sidebar lists `A`, `A`'s companion `a`, `B`, and the user
  drags `B` and releases it over `a`'s row
- **THEN** the sidebar lists `B`, `A`, `a` if the pointer was nearer the
  top of `A`'s group, and is unchanged if it was nearer the bottom

#### Scenario: Pinned rows stay at the top

- **WHEN** the user drags an agent and releases it over the Dashboard row
- **THEN** the order is unchanged and the Dashboard row stays first

### Requirement: A dragged agent row shows where it will land

While an agent row is dragged, a copy of the row SHALL follow the pointer.
It starts where the row sat, offset from the pointer by the distance at which
the row was pressed. It SHALL stay inside the window.

A horizontal insertion line SHALL mark the place the agent would be dropped,
drawn between the two rows on either side of that place, or below the last
row for the end of the list. No line SHALL be drawn while the pointer is over
a place that would leave the order unchanged: the dragged row's own place,
or the places directly above and below it. The line and the copy SHALL be
removed when the drag ends, whether or not it moved anything.

This matches the drag of a workspace row in the Workspace Manager.

#### Scenario: The line marks the drop place

- **WHEN** the sidebar lists `A`, `B`, `C` and the user drags `A` over the
  lower half of `B`
- **THEN** an insertion line is drawn between `B` and `C`

#### Scenario: No line over the dragged row's own place

- **WHEN** the user drags `B` and holds it over `B`'s own row
- **THEN** no insertion line is drawn

#### Scenario: The copy follows the pointer

- **WHEN** the user drags an agent row down the list
- **THEN** a copy of that row moves with the pointer, keeping the offset at
  which the row was pressed

### Requirement: A dragged order outlives the window

A new order made by dragging SHALL be saved when the row is dropped, so the
workspace window lists its agents in that order the next time it opens and
the next time Knot launches. A drag that ends without changing the order
SHALL NOT save anything.

#### Scenario: The order survives a relaunch

- **WHEN** the user drags `C` above `A`, quits Knot and launches it again
- **THEN** the workspace window lists `C`, `A`, `B`

#### Scenario: A drag that changes nothing saves nothing

- **WHEN** the user drags `B` and releases it over its own row
- **THEN** the order is unchanged and the workspaces file is not rewritten
