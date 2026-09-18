## Purpose

Defines the workspace sidebar's per-agent row interactions - specifically the
right-click context menu that surfaces edit, restart, companion-creation, and
removal actions the agent list otherwise has no way to reach.

## ADDED Requirements

### Requirement: Agent row context menu
Right-clicking an agent row in the workspace sidebar SHALL open a context
menu scoped to that agent, with an item set that depends on whether the
agent is a shell companion.

#### Scenario: Right-click a non-companion agent
- **WHEN** the user right-clicks a non-companion agent's row
- **THEN** the menu shows Edit Agent..., New Shell Companion, Restart Agent,
  and Remove Agent

#### Scenario: Right-click a shell companion
- **WHEN** the user right-clicks a shell companion's row
- **THEN** the menu shows Edit Agent... and Remove Agent, and does not show
  New Shell Companion or Restart Agent

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
confirmed.

#### Scenario: Confirm a restart
- **WHEN** the user selects Restart Agent and confirms the prompt
- **THEN** the agent restarts (id preserved, session cleared, state reset to
  Idle)

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
