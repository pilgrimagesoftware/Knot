# Spec Delta

## ADDED Requirements

### Requirement: The sidebar's width is set by a divider the user drags

The workspace window SHALL render a divider on the boundary between the agent
sidebar and the content column. Dragging it SHALL set the sidebar's width, the
content column taking whatever width is left.

The divider SHALL declare itself as one: it SHALL show the platform's
column-resize cursor while the pointer is over it, and SHALL be visually
distinguishable while it is being dragged. Its hit area SHALL be wider than the
line it draws, so it can be grabbed without pixel-accurate aim.

The divider SHALL NOT displace either column's contents. The sidebar hosts the
window's traffic lights and the content column's header aligns to the sidebar's
right edge; a divider that consumed layout width would misalign them.

The sidebar's width SHALL be clamped to no less than 120px and no more than
400px, and a drag that would leave the range SHALL stop at the bound rather
than being abandoned. The lower bound is above the Swift reference's 80px
because this window's traffic lights sit inside the sidebar column and reserve
80px by themselves.

The width SHALL NOT be settable any other way: no menu item, no keyboard
shortcut, and no control in the settings window.

#### Scenario: Widening the sidebar

- **WHEN** the user drags the divider to the right
- **THEN** the sidebar grows by the drag distance and the content column shrinks
  by the same amount

#### Scenario: Narrowing the sidebar

- **WHEN** the user drags the divider to the left
- **THEN** the sidebar shrinks and the content column grows

#### Scenario: The pointer says the divider can be dragged

- **WHEN** the pointer rests over the divider
- **THEN** the cursor becomes the platform's column-resize cursor

#### Scenario: Dragging past the maximum

- **WHEN** the user drags the divider far to the right, past 400px
- **THEN** the sidebar stops at 400px and the drag continues to be tracked

#### Scenario: Dragging past the minimum

- **WHEN** the user drags the divider far to the left, past 120px
- **THEN** the sidebar stops at 120px and is not hidden

#### Scenario: The content header stays aligned

- **WHEN** the sidebar is at any width
- **THEN** the content column's header starts at the sidebar's right edge, with
  no gutter between them

### Requirement: A dragged width outlives the window

A width set by dragging the divider SHALL be remembered, so a workspace window
opens at the width the user last chose rather than at the default.

The width SHALL be recorded when a drag ends, not continuously as the pointer
moves: the divider is dragged across many frames and each frame's width is not
a choice the user made.

It SHALL apply to every workspace window rather than being remembered per
workspace, and a window SHALL adopt it when it opens rather than changing width
under the user while it is open.

Recording the width SHALL NOT discard any other setting a different window has
changed since this window opened.

#### Scenario: Reopening a workspace

- **WHEN** the user drags the sidebar to 320px, closes the workspace window and
  opens it again
- **THEN** the sidebar is 320px wide

#### Scenario: A second workspace adopts the width

- **WHEN** the user drags the sidebar to 320px and then opens a different
  workspace's window
- **THEN** that window's sidebar is also 320px wide

#### Scenario: An open window is not resized underneath the user

- **WHEN** two workspace windows are open and the user drags one window's
  sidebar
- **THEN** the other window's sidebar keeps the width it is at

#### Scenario: A concurrent settings edit survives

- **WHEN** the user changes a setting in the settings window and then drags a
  workspace window's sidebar divider
- **THEN** the dragged width is recorded and the setting changed in the settings
  window is still in effect

### Requirement: A narrow sidebar shows a compact layout

Below a compact breakpoint of 160px the sidebar SHALL show a compact layout
rather than squeezing the full-width one, porting the Swift reference's
`isCompact` sidebar:

- An agent row SHALL show its avatar alone, centered, with the state dot
  overlaid on the avatar rather than in a column of its own. The name, agent
  type, companion marker, persona, activity line and folder SHALL be hidden, and
  the agent's name SHALL be available as the row's tooltip so a row stays
  identifiable.
- The dashboard row SHALL show its icon alone, centered, with its label hidden.
- The sidebar's title bar SHALL hide the application-name label and keep the
  application icon.
- The new-agent control SHALL show its icon alone, with its label hidden and its
  meaning available as a tooltip.

Everything that is not text SHALL behave as it does at full width: selection
highlighting, the dimming of an agent that is not running, the companion
indent, and every context menu and click target named elsewhere in this
capability.

At or above the breakpoint the sidebar SHALL show the full-width layout
unchanged.

#### Scenario: Crossing the breakpoint

- **WHEN** the user drags the sidebar from 250px down to 140px
- **THEN** the rows become avatar-only, the dashboard row icon-only, and the
  new-agent control icon-only

#### Scenario: Crossing back

- **WHEN** the user drags a compact sidebar back to 200px
- **THEN** every row shows its name and detail lines again

#### Scenario: A compact row is still identifiable

- **WHEN** the pointer rests on a compact agent row
- **THEN** a tooltip names that agent

#### Scenario: A compact row still reports state

- **WHEN** an agent is working and the sidebar is compact
- **THEN** its state dot is visible on its avatar

#### Scenario: A stopped agent still reads as stopped when compact

- **WHEN** a workspace holds a running and a stopped agent and the sidebar is
  compact
- **THEN** the stopped agent's row is still visually distinguishable from the
  running one's

#### Scenario: Compact rows keep their menus

- **WHEN** the user right-clicks a compact agent row
- **THEN** the agent row context menu opens with the same items it has at full
  width

#### Scenario: Selection still shows

- **WHEN** an agent is selected and the sidebar is compact
- **THEN** its row is highlighted as the selected row
