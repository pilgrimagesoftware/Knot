# Spec Delta

## MODIFIED Requirements

### Requirement: Agent row context menu
Right-clicking an agent row in the workspace sidebar SHALL open a context
menu scoped to that agent. The menu SHALL present the following items, in
this order, separated into groups by dividers:

1. New Companion…
2. New Shell Companion
   - divider -
3. Edit Agent…
4. Fork Agent
5. Duplicate Agent
   - divider -
6. Move to Workspace (submenu)
7. Save to Bench
8. Bench Agent…
   - divider -
9. Open In… (submenu)
10. Markdown Files (submenu)
   - divider -
11. Register Agent
12. Restart Agent
13. Restart with New Conversation
14. Remove Agent

Which of these a given row shows depends on the agent, per the visibility
rules below. A divider SHALL NOT render when the group it would separate
is empty, so a menu never opens with a leading, trailing, or doubled
divider.

#### Scenario: Right-click a non-companion agent
- **WHEN** the user right-clicks a non-companion, non-shell agent's row in
  a workspace that is the only attached workspace, and the agent has no
  markdown history
- **THEN** the menu shows New Companion…, New Shell Companion, Edit
  Agent…, Fork Agent, Duplicate Agent, Save to Bench, Bench Agent…, Open
  In…, Register Agent, Restart Agent, Restart with New Conversation and
  Remove Agent
- **AND** it does not show Move to Workspace (no other workspace to move
  to) or Markdown Files (no history)

#### Scenario: Right-click a shell companion
- **WHEN** the user right-clicks a shell companion's row
- **THEN** the menu shows Edit Agent…, Open In… and Remove Agent
- **AND** it does not show New Companion…, New Shell Companion, Fork
  Agent, Duplicate Agent, Move to Workspace, Save to Bench, Bench Agent…,
  Register Agent, Restart Agent or Restart with New Conversation

### Requirement: Agent row context menu visibility rules
Each item's presence SHALL be decided by the agent it was opened on:

- New Companion…, New Shell Companion, Fork Agent, Duplicate Agent, Save
  to Bench, Bench Agent… and Move to Workspace SHALL be shown only for an
  agent that is not a companion. A companion cannot own companions, be
  forked, be benched, or be moved independently of its owner.
- Move to Workspace SHALL additionally require that the agent belongs to a
  workspace and that at least one other attached workspace exists; its
  submenu SHALL list every attached workspace except the agent's own.
- Register Agent SHALL be shown only for an agent whose type is not
  `shell`. A shell agent has no coding agent to register.
- Restart Agent SHALL be shown only for an agent that is not a companion,
  per this capability's existing restart requirement.
- Restart with New Conversation SHALL be shown only for an agent that is not
  a companion and whose type is not `shell`. A shell has no conversation to
  keep or discard, so for it the item would repeat Restart Agent.
- Markdown Files SHALL be shown only when the agent has at least one entry
  in its markdown history.
- Edit Agent…, Open In… and Remove Agent SHALL always be shown.

#### Scenario: A shell agent hides Register Agent
- **WHEN** the user right-clicks a standalone shell agent's row
- **THEN** Register Agent is absent from the menu

#### Scenario: Move to Workspace needs somewhere to move to
- **WHEN** the user right-clicks an agent in the only attached workspace
- **THEN** Move to Workspace is absent from the menu
- **WHEN** a second workspace is attached and the user right-clicks the
  same agent
- **THEN** Move to Workspace is present, and its submenu lists that second
  workspace and not the agent's own

#### Scenario: A shell agent hides Restart with New Conversation
- **WHEN** the user right-clicks a standalone shell agent's row
- **THEN** Restart Agent is shown and Restart with New Conversation is absent

#### Scenario: A companion cannot be benched
- **WHEN** the user right-clicks a companion's row
- **THEN** Save to Bench and Bench Agent… are both absent

### Requirement: Sidebar background context menu

Right-clicking the workspace sidebar's agent list anywhere other than on an
agent row - the empty space below the rows, or around them - SHALL open a
context menu scoped to the workspace rather than to any one agent. The menu
SHALL present the following items, in this order:

1. New Agent
2. New from Bench (submenu)
3. Restart All
4. Close All
5. Deactivate All
   - divider -
6. Broadcast to All Agents…

Right-clicking a row SHALL still open that row's own menu, unchanged; the
background menu SHALL NOT appear over a row.

Unlike the agent row's menu, this menu SHALL show every item every time and
disable the ones that do not apply, rather than omitting them. The row menu
opens on a different row each time and is read top to bottom; this menu opens
on the same empty space every time and is learned by position, which an item
set that changes shape defeats.

Every item SHALL act on the agents of the workspace the sidebar is showing,
and on no others.

#### Scenario: Right-click the empty area below the rows

- **WHEN** the user right-clicks the empty space below the last agent row
- **THEN** a menu opens showing New Agent, New from Bench, Restart All, Close
  All, Deactivate All, a divider, and Broadcast to All Agents…

#### Scenario: Right-click a row still opens the row's menu

- **WHEN** the user right-clicks an agent row
- **THEN** that agent's row menu opens, and the background menu does not

#### Scenario: Agents in another workspace are untouched

- **WHEN** the user runs Close All in a workspace while a second workspace
  holds agents of its own
- **THEN** only the first workspace's agents are closed

### Requirement: Sidebar background menu enablement

New Agent SHALL always be enabled - an empty workspace is exactly where a
user reaches for it.

New from Bench SHALL be enabled when the bench holds at least one entry and
disabled when it is empty, whatever the workspace holds.

Restart All, Close All, Deactivate All and Broadcast to All Agents… SHALL be
disabled when the workspace holds no agents, and enabled when it holds at
least one. Deactivate All SHALL additionally be disabled when the workspace
holds agents but none of them is running, since there is then nothing to
stop.

#### Scenario: An empty workspace offers only New Agent

- **WHEN** the user right-clicks the background of a workspace with no
  agents while the bench is empty
- **THEN** New Agent is enabled, and New from Bench, Restart All, Close All,
  Deactivate All and Broadcast to All Agents… are all present and disabled

#### Scenario: An empty workspace can deploy from the bench

- **WHEN** the user right-clicks the background of a workspace with no
  agents while the bench holds an entry
- **THEN** New Agent and New from Bench are enabled

#### Scenario: An empty bench disables New from Bench

- **WHEN** the bench holds no entries
- **THEN** New from Bench is present and disabled

#### Scenario: A workspace of stopped agents cannot deactivate

- **WHEN** the workspace holds agents and none of them is running
- **THEN** Deactivate All is disabled, and Restart All, Close All and
  Broadcast to All Agents… are enabled

## ADDED Requirements

### Requirement: Bench Agent from the context menu

Selecting Bench Agent… SHALL ask for confirmation, then bench the agent per
`agent-lifecycle`'s benching requirement if confirmed. The confirmation SHALL
say that the agent is saved to the bench and closed, and that its
conversation is not kept; when the agent owns companions it SHALL also say
they are closed.

It asks where Save to Bench does not because it closes the agent: the
conversation is lost, which fails the test this menu applies to Deactivate.

#### Scenario: Confirm benching
- **WHEN** the user selects Bench Agent… and confirms
- **THEN** a bench entry for the agent's folder holds its fields, and the
  agent is removed from its workspace

#### Scenario: Cancel benching
- **WHEN** the user selects Bench Agent… and dismisses the prompt
- **THEN** the agent is left in place and the bench is unchanged

### Requirement: New from Bench from the sidebar background menu

The New from Bench submenu SHALL list every bench entry, by avatar and name,
in bench order. Selecting an entry SHALL deploy it per `agent-lifecycle`'s
bench-deployment requirement into the workspace the sidebar is showing, and
select the created agent.

When deployment fails because the entry's folder no longer exists, the entry
SHALL be removed from the bench (as that requirement says) and the user SHALL
be told which entry was removed and why, rather than the menu doing nothing
visible.

It lists the same entries as the New Agent button's bench popover (see "The
New Agent button opens the bench") and deploys them the same way; the menu is
the route that needs no pointer aim at a small chevron.

#### Scenario: Deploy an entry into this workspace
- **WHEN** the user selects a bench entry from New from Bench in a workspace
- **THEN** an agent is created from the entry in that workspace and is
  selected

#### Scenario: A stale entry is reported
- **WHEN** the user selects a bench entry whose folder no longer exists
- **THEN** no agent is created, the entry is removed from the bench, and a
  message names the entry and its missing folder

### Requirement: The New Agent button opens the bench

The new-agent control under the sidebar's agent list SHALL be a split
button, porting the Swift app's `SplitButton` with its `BenchDropdownView`:

- Its main part SHALL open the agent editor for a new agent in the
  workspace the sidebar is showing, exactly as the single button does today.
- A chevron beside it SHALL open a popover anchored to the button,
  containing, in order: a Create New Agent row (plus icon), a divider, a
  BENCH section header, and then either the bench entries or, when the bench
  is empty, a hint saying an agent is added with its row menu's Save to
  Bench or Bench Agent.
- Each bench entry row SHALL show the entry's avatar, its name, and below
  the name its folder's last path component, each truncated to one line.
  The list SHALL be in bench order and SHALL scroll once it would exceed
  300px.
- Clicking an entry row SHALL deploy it per "New from Bench from the sidebar
  background menu" - same workspace, same selection, same reporting of a
  stale entry - and close the popover. Clicking Create New Agent SHALL close
  the popover and open the agent editor, as the main part does.
- Hovering an entry row SHALL highlight it and reveal a remove control on
  it, whose tooltip says it removes the entry from the bench. Activating the
  remove control SHALL NOT deploy the entry; it SHALL ask for confirmation
  naming the entry, and on confirmation remove it from the bench, leaving
  the popover open on the updated list.
- The popover SHALL read the bench when it opens, so an entry saved from
  another window or from Settings since the window last redrew is listed.
- Escape or a click outside SHALL close the popover without acting.

In the compact sidebar layout the main part SHALL show its icon alone, as
"A narrow sidebar shows a compact layout" already requires of the
new-agent control, and the chevron SHALL remain, with its meaning available
as a tooltip.

Every control in the popover SHALL be reachable by keyboard and carry an
accessible name: the chevron, Create New Agent, each entry (by its name) and
each entry's remove control.

The Rust port differs from the Swift dropdown in reading the bench on open
rather than observing it, and in keeping the remove control keyboard
reachable rather than hover-only.

#### Scenario: The main part still creates an agent

- **WHEN** the user clicks the main part of the New Agent button
- **THEN** the agent editor opens for a new agent in that workspace, and no
  popover opens

#### Scenario: The chevron opens the bench

- **WHEN** the bench holds two entries and the user clicks the chevron
- **THEN** a popover opens showing Create New Agent, a divider, the BENCH
  header, and the two entries with avatar, name and folder name

#### Scenario: An empty bench shows how to fill it

- **WHEN** the bench is empty and the user clicks the chevron
- **THEN** the popover shows Create New Agent, the BENCH header, and the
  hint naming Save to Bench and Bench Agent

#### Scenario: Clicking an entry deploys it

- **WHEN** the user clicks a bench entry in the popover
- **THEN** an agent is created from it in the sidebar's workspace and
  selected, the popover closes, and the entry is still on the bench

#### Scenario: Removing an entry asks first

- **WHEN** the user activates a bench entry's remove control
- **THEN** a confirmation names the entry; confirming removes it from the
  bench and the popover stays open without it, and no agent is created

#### Scenario: A newly saved entry is listed

- **WHEN** an agent is saved to the bench from another window and the user
  then opens the popover
- **THEN** the new entry is listed

#### Scenario: The compact layout keeps the chevron

- **WHEN** the sidebar is narrower than the compact breakpoint
- **THEN** the New Agent button shows its icon alone and the chevron is
  still present, with a tooltip
