# quit-warning Specification

## Purpose

Protects active agent work by giving users a clear chance to cancel an
application quit before working sessions are torn down.

## Requirements

### Requirement: Quit warns about active agent work

When the application receives a quit request and one or more managed agents
are in the Working state, the system SHALL show a confirmation dialog before
terminating the application. The dialog SHALL identify that agents are still
working and provide explicit actions to quit or cancel.

#### Scenario: Quit with one working agent
- **WHEN** the user requests application quit while exactly one managed agent
  is Working
- **THEN** the application shows a warning naming one working agent and does
  not terminate until the user chooses an action

#### Scenario: Quit with multiple working agents
- **WHEN** the user requests application quit while multiple managed agents
  are Working
- **THEN** the application shows a warning containing the number of working
  agents and does not terminate until the user chooses an action

#### Scenario: Quit while all agents are idle
- **WHEN** the user requests application quit while no managed agent is
  Working
- **THEN** the application quits without showing the warning

### Requirement: Quit warning actions

The warning SHALL provide a cancel action that leaves the application and all
agent sessions running. It SHALL provide a quit action that proceeds with the
original quit request exactly once without showing the same warning again.

#### Scenario: User cancels quit
- **WHEN** the user chooses Cancel in the active-work warning
- **THEN** the dialog closes, the application remains open, and every agent
  session continues running

#### Scenario: User confirms quit
- **WHEN** the user chooses Quit in the active-work warning
- **THEN** the application terminates through the normal shutdown path without
  presenting the warning again

#### Scenario: Agent finishes while warning is open
- **WHEN** all Working agents become idle while the warning is displayed
- **THEN** the warning remains valid, and choosing Quit still terminates while
  choosing Cancel still leaves the application open
