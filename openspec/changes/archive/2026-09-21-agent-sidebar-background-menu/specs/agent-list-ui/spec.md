# Spec Delta

## ADDED Requirements

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
workspace per `agent-lifecycle`'s restart requirement - each keeping its id,
losing its session, and returning to Idle.

Dismissing the prompt without confirming SHALL leave every agent running
unchanged. Confirmation is required for the same reason the row's Restart
Agent requires it: a restart discards conversations that cannot be recovered,
and doing so to every agent at once multiplies the cost of a mis-click.

#### Scenario: Confirm a bulk restart

- **WHEN** the user selects Restart All in a workspace of three agents and
  confirms the prompt
- **THEN** all three agents restart, each keeping its id with its session
  cleared and its state reset to Idle

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
