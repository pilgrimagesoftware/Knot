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

## ADDED Requirements

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
