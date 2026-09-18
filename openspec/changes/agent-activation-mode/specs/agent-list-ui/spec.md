## ADDED Requirements

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
