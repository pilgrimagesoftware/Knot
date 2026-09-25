# agent-list-ui Specification

## Purpose

Defines the workspace sidebar's per-agent row interactions - specifically the
right-click context menu that surfaces edit, restart, companion-creation, and
removal actions the agent list otherwise has no way to reach.

## Requirements

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
   - divider -
8. Open In… (submenu)
9. Markdown Files (submenu)
   - divider -
10. Register Agent
11. Restart Agent
12. Restart with New Conversation
13. Remove Agent

Which of these a given row shows depends on the agent, per the visibility
rules below. A divider SHALL NOT render when the group it would separate
is empty, so a menu never opens with a leading, trailing, or doubled
divider.

#### Scenario: Right-click a non-companion agent
- **WHEN** the user right-clicks a non-companion, non-shell agent's row in
  a workspace that is the only attached workspace, and the agent has no
  markdown history
- **THEN** the menu shows New Companion…, New Shell Companion, Edit
  Agent…, Fork Agent, Duplicate Agent, Save to Bench, Open In…, Register
  Agent, Restart Agent, Restart with New Conversation and Remove Agent
- **AND** it does not show Move to Workspace (no other workspace to move
  to) or Markdown Files (no history)

#### Scenario: Right-click a shell companion
- **WHEN** the user right-clicks a shell companion's row
- **THEN** the menu shows Edit Agent…, Open In… and Remove Agent
- **AND** it does not show New Companion…, New Shell Companion, Fork
  Agent, Duplicate Agent, Move to Workspace, Save to Bench, Register Agent,
  Restart Agent or Restart with New Conversation

### Requirement: Edit Agent from the context menu
Selecting Edit Agent... SHALL open the same agent editor dialog already
reachable elsewhere in the UI, pre-populated for the selected agent.

#### Scenario: Edit from the context menu
- **WHEN** the user selects Edit Agent... for a row
- **THEN** the agent editor dialog opens for that agent's id

### Requirement: New Shell Companion from the context menu
Selecting New Shell Companion on a non-companion agent SHALL create a shell
companion bound to that agent, per `agent-lifecycle`'s companion-creation
requirement (placed after its owner, split layout pairing owner and
companion).

#### Scenario: Create a companion
- **WHEN** the user selects New Shell Companion for a non-companion agent
- **THEN** a new shell companion agent is created with that agent as its
  owner, and the two appear in a split layout

### Requirement: Restart Agent from the context menu
Selecting Restart Agent on a non-companion agent SHALL prompt for
confirmation, then restart it per `agent-lifecycle`'s restart requirement if
confirmed. It keeps the agent's conversation when
`restore-conversation-on-launch` is enabled and starts a new one when it is
disabled, and the prompt SHALL say which of the two it will do.

#### Scenario: Confirm a restart
- **WHEN** the user selects Restart Agent and confirms the prompt
- **THEN** the agent restarts (id preserved, state reset to Idle), keeping its
  conversation if `restore-conversation-on-launch` is enabled and clearing its
  session otherwise

#### Scenario: The prompt says whether the conversation is kept
- **WHEN** `restore-conversation-on-launch` is enabled and the user selects
  Restart Agent
- **THEN** the prompt says the agent will resume its current conversation

#### Scenario: Cancel a restart
- **WHEN** the user selects Restart Agent and dismisses the prompt without
  confirming
- **THEN** the agent is left running unchanged

### Requirement: Remove Agent from the context menu
Selecting Remove Agent SHALL prompt for confirmation, then remove the agent
(and, if it owns any, its companions) per `agent-lifecycle`'s removal
requirement if confirmed.

#### Scenario: Confirm a removal
- **WHEN** the user selects Remove Agent and confirms the prompt
- **THEN** the agent (and any companions it owns) is removed from its
  workspace and the master agent list, and its terminal session is torn down

#### Scenario: Cancel a removal
- **WHEN** the user selects Remove Agent and dismisses the prompt without
  confirming
- **THEN** the agent is left in place unchanged

### Requirement: Agent row context menu visibility rules
Each item's presence SHALL be decided by the agent it was opened on:

- New Companion…, New Shell Companion, Fork Agent, Duplicate Agent, Save
  to Bench and Move to Workspace SHALL be shown only for an agent that is
  not a companion. A companion cannot own companions, be forked, or be
  moved independently of its owner.
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

### Requirement: New Companion from the context menu
Selecting New Companion… SHALL open the agent editor to create a
companion of the selected agent, pre-populated with the owner's folder, a
`shell` agent type, and placement immediately after the owner. It differs
from New Shell Companion in that the user confirms or edits the new
agent's details before it is created.

#### Scenario: Configure a companion before creating it
- **WHEN** the user selects New Companion… for a non-companion agent
- **THEN** the agent editor opens pre-populated with that agent's folder,
  agent type `shell`, and the agent as its owner
- **AND** no agent exists until the editor is submitted

### Requirement: Fork Agent from the context menu
Selecting Fork Agent SHALL open the agent editor to create a new agent
carrying the selected agent's folder, avatar, agent type and persona, its
session id (so the fork resumes the same conversation), a name suffixed to
mark it as a fork, and placement immediately after the source.

#### Scenario: Fork carries the session forward
- **WHEN** the user selects Fork Agent on an agent with session id `s1`
- **THEN** the agent editor opens pre-populated from that agent, carrying
  `s1`, and the created agent is placed immediately after the source

#### Scenario: Cancelling a fork creates nothing
- **WHEN** the user selects Fork Agent and dismisses the editor
- **THEN** no agent is created

### Requirement: Duplicate Agent from the context menu
Selecting Duplicate Agent SHALL immediately create a copy of the selected
agent - same folder, avatar, agent type and persona, a name marking it as
a copy - placed immediately after the source. Unlike Fork Agent it SHALL
NOT carry the source's session, and SHALL NOT open the editor first.

#### Scenario: Duplicate creates a fresh agent at once
- **WHEN** the user selects Duplicate Agent
- **THEN** a new agent is created immediately, after the source, sharing
  its folder, avatar, type and persona, with no session id of its own

### Requirement: Move to Workspace from the context menu
Selecting a workspace from the Move to Workspace submenu SHALL move the
agent out of its current workspace and into the chosen one, leaving the
source workspace with a valid selection.

#### Scenario: Moving an agent between workspaces
- **WHEN** the user moves agent `A` from workspace `W1` to `W2`
- **THEN** `A` is no longer in `W1`'s agents and is in `W2`'s
- **AND** if `A` was `W1`'s only selected agent, `W1` selects another of
  its agents, or none if it now has none

### Requirement: Save to Bench from the context menu
Selecting Save to Bench SHALL store the agent on the bench for later
redeployment, replacing any existing bench entry for the same folder so
the bench never holds two entries for one folder.

#### Scenario: Saving replaces an entry for the same folder
- **WHEN** the user saves an agent whose folder already has a bench entry
- **THEN** the bench holds exactly one entry for that folder, the new one

### Requirement: Open In from the context menu
The Open In… submenu SHALL list VS Code, Zed, Xcode, a divider, Finder and
Terminal, and selecting one SHALL open the agent's folder in that
application. A missing application SHALL fail quietly rather than
reporting an error the user cannot act on.

#### Scenario: Open the agent's folder in an editor
- **WHEN** the user selects VS Code from Open In…
- **THEN** the agent's folder is opened in VS Code

#### Scenario: Open the agent's folder in Zed
- **WHEN** the user selects Zed from Open In…
- **THEN** the agent's folder is opened in Zed

#### Scenario: The chosen application is not installed
- **WHEN** the user selects an application that is not installed
- **THEN** nothing opens and the app does not present an error dialog

### Requirement: Markdown Files from the context menu
The Markdown Files submenu SHALL list the agent's markdown history - the
files shown through the `display-markdown` MCP tool - most recent first,
labelled by file name rather than full path. Selecting one SHALL display
that file for the agent.

#### Scenario: Re-open a previously shown markdown file
- **WHEN** an agent has shown `docs/plan.md` and `README.md` through the
  MCP tool, and the user selects `plan.md` from Markdown Files
- **THEN** that file is displayed for the agent

### Requirement: Register Agent from the context menu
Selecting Register Agent SHALL send the agent its MCP registration prompt
over whichever channel that agent is driven by - its panel session for an
ACP-launched agent, its terminal for one running in a terminal - so a
coding agent that failed to register at launch can be registered by hand
without restarting it.

#### Scenario: Registering an ACP agent by hand
- **WHEN** the user selects Register Agent on a connected ACP agent
- **THEN** the registration prompt is sent as a prompt in that agent's
  panel session, and appears in its conversation like any other prompt

#### Scenario: Registering an agent that is not connected
- **WHEN** the user selects Register Agent on an agent whose session is
  not live
- **THEN** nothing is sent, and the agent's pane goes on showing the
  connection state that already explains why - the menu SHALL NOT stack a
  second message on top of it

#### Scenario: The agent refuses the registration prompt
- **WHEN** a connected agent answers the registration prompt with an error
- **THEN** that failure appears in the agent's conversation, per
  `acp-panel-ui`'s handling of a refused prompt

### Requirement: Every detail line on an agent row is labelled by an icon

An agent row's detail lines - the ones under the agent's name - SHALL each
carry a leading icon identifying what that line is: its agent type, its
persona, its status, and its folder.

Without one, a line is a bare string whose meaning has to be inferred from
its content, which fails exactly when it matters: an agent whose status text
happens to look like a path, or whose folder is named after a person, reads
as the wrong thing entirely.

The icons SHALL be drawn as icons rather than as characters inside the line's
text, so they share the lines' muted colour, size with their line, and sit in
one column down the row.

#### Scenario: A status line is identifiable as a status

- **WHEN** an agent row renders with a status
- **THEN** that line carries a leading icon marking it as the status

#### Scenario: A folder line is identifiable as a folder

- **WHEN** an agent row renders its folder
- **THEN** that line carries a leading icon marking it as a folder

#### Scenario: The icons line up

- **WHEN** an agent row renders with a type, a persona, a status and a folder
- **THEN** all four leading icons share the row's muted colour and align in
  one column

#### Scenario: A line with nothing to show has no icon

- **WHEN** an agent has no persona
- **THEN** no persona line and no persona icon are drawn, rather than an icon
  beside an empty line

### Requirement: A detail line's icon is sized to its own line

Each detail line's icon SHALL be sized to that line's text, not to a single
fixed size across the row. The row's lines do not all render at the same text
size, and an icon fixed to one of them reads as undersized or oversized
beside the others.

#### Scenario: Icons match their lines

- **WHEN** a row renders detail lines at more than one text size
- **THEN** each line's icon matches the text beside it

#### Scenario: A line that wraps keeps its icon beside its first line

- **WHEN** a detail line is long enough to wrap onto a second line - a
  two-word persona name, say
- **THEN** its icon sits beside the first line, not centred against the
  block as a whole

### Requirement: Deactivate from the context menu

The agent row's context menu SHALL offer Deactivate for an agent that is
currently running, in the group that holds Restart Agent and Remove Agent and
immediately before Restart Agent. Selecting it SHALL deactivate that agent per
`agent-lifecycle`'s deactivation requirement.

Deactivate SHALL be absent for an agent that is not running - there is nothing
to stop - rather than shown disabled, matching how this menu hides every other
item that does not apply.

Deactivate SHALL NOT ask for confirmation. Nothing is lost that the agent
cannot get back by being selected again, which is the test this menu applies
to Restart and Remove and which those two fail.

#### Scenario: Deactivate a running agent from its row

- **WHEN** the user right-clicks a running agent and selects Deactivate
- **THEN** the agent is deactivated, with no confirmation prompt

#### Scenario: A stopped agent has nothing to deactivate

- **WHEN** the user right-clicks a `passive` agent that has never been
  selected, or one that is already deactivated
- **THEN** Deactivate is absent from the menu

#### Scenario: Deactivate sits with the other session actions

- **WHEN** the menu is opened on a running non-companion agent
- **THEN** Deactivate appears immediately above Restart Agent, in the same
  group as Restart Agent and Remove Agent

### Requirement: A stopped agent is distinguishable in the sidebar

An agent that is not running SHALL be visually distinguishable in the
sidebar from one that is, so a workspace of mixed agents can be read at a
glance without opening each row.

The distinction SHALL apply to any agent that is not running, whether it is
passive and never started or was deactivated - what the user needs to know
is which agents are live, not why each one is not.

It SHALL NOT rely on the agent's state dot, which reports what a *running*
agent is doing (idle, working, awaiting input, error) and has no value that
means "not running at all".

#### Scenario: A passive agent that has never started

- **WHEN** a workspace holds a running agent and a passive agent that has
  not been selected
- **THEN** the two rows are visually distinguishable, and the passive one
  reads as not running

#### Scenario: Activating a passive agent brings its row up to the others

- **WHEN** the user selects that passive agent and it starts
- **THEN** its row becomes indistinguishable from any other running agent's

#### Scenario: A deactivated agent reads the same as one never started

- **WHEN** a running agent is deactivated
- **THEN** its row takes the same not-running appearance as a passive agent
  that never started

### Requirement: Sidebar background context menu

Right-clicking the workspace sidebar's agent list anywhere other than on an
agent row - the empty space below the rows, or around them - SHALL open a
context menu scoped to the workspace rather than to any one agent. The menu
SHALL present the following items, in this order:

1. New Agent
2. Restart All
3. Close All
4. Deactivate All
   - divider -
5. Broadcast to All Agents…

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
- **THEN** a menu opens showing New Agent, Restart All, Close All,
  Deactivate All, a divider, and Broadcast to All Agents…

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

Restart All, Close All, Deactivate All and Broadcast to All Agents… SHALL be
disabled when the workspace holds no agents, and enabled when it holds at
least one. Deactivate All SHALL additionally be disabled when the workspace
holds agents but none of them is running, since there is then nothing to
stop.

#### Scenario: An empty workspace offers only New Agent

- **WHEN** the user right-clicks the background of a workspace with no
  agents
- **THEN** New Agent is enabled, and Restart All, Close All, Deactivate All
  and Broadcast to All Agents… are all present and disabled

#### Scenario: A workspace of stopped agents cannot deactivate

- **WHEN** the workspace holds agents and none of them is running
- **THEN** Deactivate All is disabled, and Restart All, Close All and
  Broadcast to All Agents… are enabled

### Requirement: New Agent from the sidebar background menu

Selecting New Agent SHALL open the agent editor to create a new agent in the
workspace the sidebar is showing - the same dialog the sidebar's existing
"New agent" button opens, with the same defaults.

#### Scenario: Create an agent from the background menu

- **WHEN** the user selects New Agent
- **THEN** the agent editor opens for a new agent in that workspace
- **AND** dismissing it without submitting creates no agent

### Requirement: Restart All from the sidebar background menu

Selecting Restart All SHALL prompt for confirmation, naming how many agents
would be restarted, and on confirmation SHALL restart every agent in the
workspace per `agent-lifecycle`'s restart requirement - each keeping its id
and returning to Idle, and each keeping its conversation when
`restore-conversation-on-launch` is enabled or losing it when it is
disabled, the same as Restart Agent.

Dismissing the prompt without confirming SHALL leave every agent running
unchanged. Confirmation is required for the same reason the row's Restart
Agent requires it: a restart discards conversations that cannot be recovered,
and doing so to every agent at once multiplies the cost of a mis-click.

#### Scenario: Confirm a bulk restart

- **WHEN** the user selects Restart All in a workspace of three agents and
  confirms the prompt
- **THEN** all three agents restart, each keeping its id with its state reset
  to Idle, and each keeping or clearing its session per
  `restore-conversation-on-launch`

#### Scenario: The prompt says how many agents it would restart

- **WHEN** the user selects Restart All in a workspace of three agents
- **THEN** the confirmation names that it would restart three agents

#### Scenario: Cancel a bulk restart

- **WHEN** the user selects Restart All and dismisses the prompt without
  confirming
- **THEN** every agent is left running unchanged

### Requirement: Close All from the sidebar background menu

Selecting Close All SHALL prompt for confirmation, naming how many agents
would be closed, and on confirmation SHALL remove every agent in the
workspace - and any companions they own - per `agent-lifecycle`'s removal
requirement, tearing down each agent's session and leaving the workspace
empty.

Dismissing the prompt without confirming SHALL leave every agent in place.

Close All removes agents permanently; it is not a pause. Deactivate All is
the reversible member of this pair.

#### Scenario: Confirm a bulk close

- **WHEN** the user selects Close All and confirms the prompt
- **THEN** every agent in the workspace, and every companion those agents
  own, is removed from the workspace and from the master agent list, and
  each agent's session is torn down
- **AND** the workspace's sidebar shows no agents

#### Scenario: Cancel a bulk close

- **WHEN** the user selects Close All and dismisses the prompt without
  confirming
- **THEN** every agent is left in place unchanged

### Requirement: Deactivate All from the sidebar background menu

Selecting Deactivate All SHALL deactivate every running agent in the
workspace per `agent-lifecycle`'s deactivation requirement. Agents that are
not running SHALL be left alone rather than treated as a failure.

Deactivate All SHALL NOT ask for confirmation, matching the row menu's
Deactivate: every agent it stops comes back by being selected, which is the
test Restart All and Close All fail.

After it runs, every agent SHALL still be listed in the sidebar, each
rendered as not running per this capability's existing requirement that a
stopped agent is distinguishable.

This item has no counterpart in the Swift reference, which has no deactivate
concept at any level. It is the bulk form of the port's own per-agent
Deactivate.

#### Scenario: Deactivate every running agent at once

- **WHEN** the user selects Deactivate All in a workspace of three running
  agents
- **THEN** all three are deactivated, with no confirmation prompt
- **AND** all three are still listed in the sidebar, each reading as not
  running

#### Scenario: A mixed workspace deactivates only what is running

- **WHEN** the workspace holds two running agents and one passive agent
  that has never been selected
- **THEN** the two running agents are deactivated and the passive one is
  left as it was

### Requirement: Broadcast to All Agents from the sidebar background menu

Selecting Broadcast to All Agents… SHALL open a message sheet holding a
multi-line text field, a Cancel button and a Send button, starting empty each
time it opens rather than carrying the previous message forward.

Send SHALL be disabled while the message is empty or consists only of
whitespace. Escape SHALL cancel the sheet, discarding the message. Because
the field is multi-line, Return SHALL insert a newline rather than sending;
the sheet SHALL offer a modifier-Return shortcut for Send instead, as the
reference does.

On send, the trimmed message SHALL be delivered to every agent in the
workspace the same way the user typing that text into that agent's own
composer would deliver it: as a prompt to an agent driven by an ACP panel
session, as injected text to one running in a terminal. A panel agent that is
mid-turn or holding a permission prompt SHALL have the message queued behind
its current turn, per `acp-panel-ui`'s prompt-queueing requirement - the
broadcast SHALL NOT be dropped simply because one agent was busy when it was
sent, and SHALL NOT interrupt a turn in progress.

An agent with no live session at all - one never activated, or one whose
session has gone - SHALL be skipped quietly rather than failing the broadcast
for the others. One unreachable agent is not a reason to withhold the message
from the rest.

This is deliberately not `mcp-messaging`'s broadcast, which places a message
in each agent's inbox for the agent to read with `check-messages`. That one
is agents talking to each other; this one is the user talking to their
agents, and the two arrive differently on purpose.

#### Scenario: Broadcast reaches every agent

- **WHEN** the user opens the broadcast sheet in a workspace of three
  running agents, types a message, and sends it
- **THEN** all three agents receive that message as though the user had
  typed it into each one
- **AND** the sheet closes

#### Scenario: An empty message cannot be sent

- **WHEN** the broadcast sheet is open with an empty or whitespace-only
  message
- **THEN** Send is disabled

#### Scenario: Escape discards the message

- **WHEN** the user has typed a message and presses Escape
- **THEN** the sheet closes and nothing is sent

#### Scenario: The sheet does not remember the last message

- **WHEN** the user sends a broadcast and then opens the sheet again
- **THEN** the message field is empty

#### Scenario: A busy agent gets the message queued, not dropped

- **WHEN** the workspace holds one idle panel agent and one mid-turn, and
  the user sends a broadcast
- **THEN** the idle agent receives it at once, the mid-turn agent's turn is
  not interrupted, and the message is queued for it and delivered when that
  turn ends

#### Scenario: An agent that is not connected is skipped

- **WHEN** the workspace holds one running agent and one that has no live
  session at all, and the user sends a broadcast
- **THEN** the running agent receives the message and the other is skipped,
  with no error presented

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

### Requirement: The sidebar's title bar names the workspace

The sidebar's title bar — the strip that holds the workspace window's traffic
lights — SHALL show the application icon followed by the name of the workspace
the window belongs to. It SHALL NOT show the application's name: a workspace
window is one of several the user may have open, and the one thing that tells
them apart is the workspace.

This diverges from the Swift reference, whose workspace window shows the
application name here. It aligns the window with its siblings, which already
name themselves in the same position — the Command Center and the Workspaces
window — and with this window's own OS-level title, which already carries the
workspace name and is what Mission Control, Cmd+` and the Window menu show.

The name SHALL be the workspace's current name, not the name it had when the
window opened: renaming a workspace SHALL update the title bar of that
workspace's open window, and that window's OS title, without the window being
closed and reopened, and SHALL leave every other open window's title unchanged.

The name SHALL occupy one line. A name too long for the available width SHALL
be truncated with an ellipsis rather than wrapping, pushing the title bar
taller, or displacing the traffic lights.

#### Scenario: The window says which workspace it is

- **WHEN** a workspace window is open for a workspace named "Payments"
- **THEN** its title bar shows the application icon followed by "Payments", and
  does not show the application's name

#### Scenario: Two open windows are distinguishable

- **WHEN** windows are open for two different workspaces
- **THEN** each title bar shows its own workspace's name

#### Scenario: A rename reaches the open window

- **WHEN** the user renames a workspace in the Workspaces window while that
  workspace's window is open
- **THEN** the open window's title bar shows the new name, and so does its OS
  window title, with no reopen

#### Scenario: A rename leaves other windows alone

- **WHEN** the user renames one workspace while windows for two workspaces are
  open
- **THEN** only the renamed workspace's window changes its title

#### Scenario: A long name is truncated

- **WHEN** the workspace's name is wider than the space the title bar has for it
- **THEN** the name is truncated with an ellipsis on one line, and the traffic
  lights stay where they are

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
- The sidebar's title bar SHALL hide the workspace name and keep the application
  icon. The workspace SHALL stay identifiable while the label is hidden: the
  window's OS title still carries the name, and the title bar SHALL offer the
  name as a tooltip, the same way a compact agent row does.
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

#### Scenario: The compact title bar is still identifiable

- **WHEN** the sidebar is compact and the pointer rests on its title bar
- **THEN** a tooltip names the workspace

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

### Requirement: Restart with New Conversation from the context menu
Selecting Restart with New Conversation on a non-companion agent SHALL prompt
for confirmation, then restart it starting a new conversation, per
`agent-lifecycle`'s restart requirement, whatever
`restore-conversation-on-launch` is set to. The prompt SHALL say that the
current conversation will not be resumed.

It is the way to get a clean session once Restart Agent keeps the
conversation. With `restore-conversation-on-launch` disabled it does the same
as Restart Agent, and it is shown anyway, so the menu keeps one shape whatever
the setting.

#### Scenario: Start over with restore on
- **WHEN** `restore-conversation-on-launch` is enabled and the user selects
  Restart with New Conversation and confirms
- **THEN** the agent restarts in a new session, with its initialization
  prompt, and its previous conversation is not loaded

#### Scenario: Cancel starting over
- **WHEN** the user selects Restart with New Conversation and dismisses the
  prompt
- **THEN** the agent is left running unchanged

### Requirement: Holding ⌘ shows the sidebar's shortcuts

While the user holds ⌘ in a focused workspace window, the sidebar SHALL show
the key that reaches each of these controls, beside it:

| Control | Hint |
| --- | --- |
| Dashboard row | the Toggle Dashboard shortcut (default ⌥⌘O) |
| Pull Requests row | the Toggle Pull Requests shortcut (default ⌥⌘P) |
| The Nth agent row, N from 1 to 9 | the Select agent N shortcut (`keybindings`) |
| New agent control | the New Agent shortcut (⌘T) |

Each hint SHALL show the binding currently in effect, in the glyph form macOS
menus use, and SHALL follow a rebinding without a restart. Agent rows after
the ninth have no shortcut and SHALL show no hint.

The hints SHALL appear only after ⌘ has been held for 500 ms with no other key
pressed, so that an ordinary ⌘ shortcut such as ⌘C does not flash them. They
SHALL disappear as soon as ⌘ is released, a non-modifier key is pressed, or the
window stops being the key window. Adding ⌥, ⌃ or ⇧ while ⌘ is held SHALL NOT
hide them, because several shortcuts need ⌥ as well.

Hints are shown whether or not the binding includes ⌘. ⌘ is the trigger
because every default binding holds it.

A hint SHALL NOT move or resize the control it labels, or any other row: at
full width it takes the right end of the row, over the row's own trailing
content. In the compact layout a row's hint SHALL be drawn as a badge over the
corner of its avatar or icon. The New agent control's hint sits against its
trailing edge at both widths.

The hints are not a Swift reference feature. The Swift sidebar shows no
shortcuts.

#### Scenario: Hints appear while ⌘ is held

- **WHEN** a workspace window lists agents X, Y and Z, no shortcut is
  customized, and the user holds ⌘ for a second
- **THEN** the Dashboard row shows ⌥⌘O, the Pull Requests row ⌥⌘P, X, Y and Z
  the default Select agent 1, 2 and 3 keys, and the new agent control ⌘T

#### Scenario: A quick shortcut does not flash them

- **WHEN** the user presses ⌘C and releases both keys within 200 ms
- **THEN** no hint appears

#### Scenario: Releasing ⌘ hides them

- **WHEN** the hints are showing and the user releases ⌘
- **THEN** every hint disappears

#### Scenario: Adding ⌥ keeps them

- **WHEN** the hints are showing and the user also presses ⌥
- **THEN** the hints stay

#### Scenario: Switching away hides them

- **WHEN** the hints are showing and the user presses ⌘Tab to another
  application
- **THEN** no hint remains when the user returns to the window

#### Scenario: A rebinding shows

- **WHEN** the user rebinds Toggle Dashboard to ⌃⌘D and holds ⌘
- **THEN** the Dashboard row shows ⌃⌘D

#### Scenario: A tenth agent

- **WHEN** a workspace has ten agents and the user holds ⌘
- **THEN** the first nine rows show a hint and the tenth shows none

#### Scenario: Compact layout

- **WHEN** the sidebar is compact and the user holds ⌘
- **THEN** each hint is a badge over its avatar or icon, and no row changes size

#### Scenario: Another window

- **WHEN** the Command Center is focused and the user holds ⌘
- **THEN** no workspace window's sidebar shows hints
