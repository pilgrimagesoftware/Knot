## Purpose

Delivers macOS desktop notifications (via `UNUserNotificationCenter`) when an
agent needs the user's attention while Knot is backgrounded or hidden to the
menu bar, and routes a click on that notification back to the agent.

## Requirements

### Requirement: Authorization requested once at startup

The system SHALL request notification authorization (alert and sound) once
during application startup, regardless of the `desktop_notifications_enabled`
setting's value at that time. A denied or not-yet-answered authorization
SHALL NOT block startup or raise an error.

#### Scenario: Authorization requested on launch

- **WHEN** the application starts
- **THEN** a notification authorization request is issued exactly once

### Requirement: Awaiting-input raises a desktop notification

When an agent enters Awaiting input and `desktop_notifications_enabled` is
on, the system SHALL raise a desktop notification titled with the agent's
name and bodied with the hook-supplied message, or "Needs your attention"
when no message is present. The notification SHALL be delivered immediately
and carry the agent's id so a click can route back to it.

#### Scenario: Notification raised with hook message

- **WHEN** agent "auth-service" enters Awaiting input with hook message
  "Grant filesystem access?" and the setting is on
- **THEN** a desktop notification titled "Knot - auth-service" with body
  "Grant filesystem access?" is raised

#### Scenario: Notification uses default body without a message

- **WHEN** an agent enters Awaiting input with no hook-supplied message and
  the setting is on
- **THEN** the notification body is "Needs your attention"

#### Scenario: Setting off suppresses the notification

- **WHEN** an agent enters Awaiting input and `desktop_notifications_enabled`
  is off
- **THEN** no desktop notification is raised

### Requirement: Repeat and visible-agent suppression

The system SHALL NOT raise a duplicate notification for an agent already in
Awaiting input (a second hook event for the same prompt), and SHALL NOT raise
a notification for an agent that is currently the selected/visible agent in
the app's active workspace.

#### Scenario: Second hook event for the same prompt is suppressed

- **WHEN** an agent already in Awaiting input receives another hook event
  reporting Awaiting input
- **THEN** no additional notification is raised

#### Scenario: Visible agent is suppressed

- **WHEN** the agent entering Awaiting input is the currently selected agent
  in the active workspace
- **THEN** no desktop notification is raised

### Requirement: Clicking a notification raises the app

The system SHALL respond to a notification click carrying a valid agent id
by bringing the application to the front. A click whose tag does not parse
as an agent id SHALL be a no-op.

Selecting the specific clicked agent (in whichever workspace window owns it)
is not yet implemented: no registry exists mapping an agent id to its owning
window across the port's per-workspace `Shell` windows. That refinement is
tracked as a follow-up and is not required by this version of the
requirement.

#### Scenario: Click with a valid agent id raises the app

- **WHEN** the user clicks a delivered notification whose tag is agent `X`'s
  id
- **THEN** the application is brought to the front

#### Scenario: Click with an unparseable tag is a no-op

- **WHEN** the user clicks a notification whose tag does not parse as a
  valid agent id
- **THEN** nothing changes and no error is raised

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
