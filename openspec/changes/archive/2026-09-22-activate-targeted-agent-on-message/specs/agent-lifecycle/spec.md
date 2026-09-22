# Spec Delta

## MODIFIED Requirements

### Requirement: Deactivating an agent

The system SHALL let a running agent be deactivated: its terminal or ACP
session is torn down and its pane shows that it is stopped, while the agent
itself remains in its workspace with its name, folder, ordering, persona and
activation mode intact.

Deactivating SHALL NOT remove the agent, SHALL NOT remove its companions, and
SHALL NOT take it out of any workspace. A deactivated agent SHALL NOT start
again on its own for as long as its workspace stays open, with two
exceptions: selecting it starts it, whatever its activation mode, and a
direct message addressed to it (see `mcp-messaging`'s "Direct send activates
a deactivated recipient") starts it the same way. Nothing else - a broadcast
reaching it, another agent's session starting, a poll or a timer - starts a
deactivated agent.

Deactivating an agent SHALL first deactivate every companion it owns, since a
companion has no session of its own to keep once its owner's is gone.

#### Scenario: Deactivate a running agent

- **WHEN** the user deactivates a running agent
- **THEN** its session is torn down, it remains in its workspace in the same
  position, and its pane reports that it is stopped

#### Scenario: A deactivated agent does not restart by itself

- **WHEN** an `active` agent is deactivated and the user selects other agents
  and returns to looking at the sidebar
- **THEN** that agent stays stopped until it is selected again

#### Scenario: Selecting a deactivated agent starts it

- **WHEN** the user selects a deactivated agent
- **THEN** it starts, in either activation mode

#### Scenario: Deactivating an owner takes its companions with it

- **WHEN** an agent owning two shell companions is deactivated
- **THEN** both companions are deactivated first, then the owner

#### Scenario: A direct message starts a deactivated agent

- **WHEN** another agent in the same workspace sends a direct message to a
  deactivated agent
- **THEN** the deactivated agent starts, in either activation mode

#### Scenario: A broadcast does not start a deactivated agent

- **WHEN** another agent broadcasts to its workspace and a deactivated agent
  is among the eligible recipients
- **THEN** that agent stays stopped
