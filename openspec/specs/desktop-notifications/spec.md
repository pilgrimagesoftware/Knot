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
name and bodied with the attention message that accompanied the transition, or
"Needs your attention" when none is present. The notification SHALL be delivered
immediately and carry the agent's id so a click can route back to it.

The attention message SHALL come from whichever route entered the state: the
hook-supplied message for a hook-reported transition, and the permission
request's own prompt text for an ACP transition (see `activity-detection`'s
"ACP updates drive status for Panel-mode agents"). This requirement SHALL hold
for every agent that enters Awaiting input, whatever its view mode and whatever
route reported it.

#### Scenario: Notification raised with hook message

- **WHEN** agent "auth-service" enters Awaiting input with hook message
  "Grant filesystem access?" and the setting is on
- **THEN** a desktop notification titled "Knot - auth-service" with body
  "Grant filesystem access?" is raised

#### Scenario: Notification raised for a Panel-mode agent

- **WHEN** a Panel-mode agent enters Awaiting input because it sent a permission
  request reading "Allow writing to src/main.rs?" and the setting is on
- **THEN** a desktop notification for that agent is raised with body
  "Allow writing to src/main.rs?"

#### Scenario: Notification uses default body without a message

- **WHEN** an agent enters Awaiting input with no accompanying message and
  the setting is on
- **THEN** the notification body is "Needs your attention"

#### Scenario: Setting off suppresses the notification

- **WHEN** an agent enters Awaiting input and `desktop_notifications_enabled`
  is off
- **THEN** no desktop notification is raised

### Requirement: Repeat and visible-agent suppression

The system SHALL NOT raise a duplicate notification for an agent already in
Awaiting input, whatever route reports it again — a second hook event for the
same prompt, or a repeated report of a permission request that is still pending —
and SHALL NOT raise a notification for an agent that is currently the
selected/visible agent in the app's active workspace.

A status source that reports continuously rather than on transitions SHALL NOT
produce a notification per report. Only entering Awaiting input from another
status SHALL raise one.

#### Scenario: Second hook event for the same prompt is suppressed

- **WHEN** an agent already in Awaiting input receives another hook event
  reporting Awaiting input
- **THEN** no additional notification is raised

#### Scenario: A still-pending permission request is not re-notified

- **WHEN** a Panel-mode agent's permission request is still pending several
  status reports later
- **THEN** no additional notification is raised for it

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
