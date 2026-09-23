# Spec Delta

## ADDED Requirements

### Requirement: The MCP server's failure is notified

When the MCP server enters a failing state — it could not be started, or it failed while
running — and `desktop_notifications_enabled` is on, the system SHALL raise a desktop
notification identifying the MCP server and stating that agents cannot reach Knot until it
recovers.

The notification SHALL be raised once per failure episode, not once per retry attempt: repeated
failed attempts within one episode SHALL NOT produce further notifications. An episode ends when
the server reaches running again, and a later failure SHALL notify again.

This notification is not tied to an agent. A click on it SHALL bring the application to the
front, and SHALL NOT attempt to select an agent.

#### Scenario: A failure notifies once

- **WHEN** the MCP server fails and then retries several times without succeeding
- **THEN** exactly one notification is raised for that episode

#### Scenario: A later failure notifies again

- **WHEN** the server recovers to running and subsequently fails again
- **THEN** a further notification is raised

#### Scenario: Notifications respect the setting

- **WHEN** the MCP server fails while `desktop_notifications_enabled` is off
- **THEN** no notification is raised, and the settings pane's state row still reports the
  failure

#### Scenario: Clicking the notification

- **WHEN** the user clicks the MCP server failure notification
- **THEN** the application comes to the front and no agent selection is attempted
