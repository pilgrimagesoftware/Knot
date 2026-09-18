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
   - divider -
8. Open In… (submenu)
9. Markdown Files (submenu)
   - divider -
10. Register Agent
11. Restart Agent
12. Remove Agent

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
  Agent, Restart Agent and Remove Agent
- **AND** it does not show Move to Workspace (no other workspace to move
  to) or Markdown Files (no history)

#### Scenario: Right-click a shell companion
- **WHEN** the user right-clicks a shell companion's row
- **THEN** the menu shows Edit Agent…, Open In… and Remove Agent
- **AND** it does not show New Companion…, New Shell Companion, Fork
  Agent, Duplicate Agent, Move to Workspace, Save to Bench, Register Agent
  or Restart Agent

## ADDED Requirements

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
The Open In… submenu SHALL list VS Code, Xcode, a divider, Finder and
Terminal, and selecting one SHALL open the agent's folder in that
application. A missing application SHALL fail quietly rather than
reporting an error the user cannot act on.

#### Scenario: Open the agent's folder in an editor
- **WHEN** the user selects VS Code from Open In…
- **THEN** the agent's folder is opened in VS Code

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
