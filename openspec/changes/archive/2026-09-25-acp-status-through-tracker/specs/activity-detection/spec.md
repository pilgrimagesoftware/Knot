# Spec Delta

## MODIFIED Requirements

### Requirement: ACP updates drive status for Panel-mode agents

For an agent tracking `acp-updates`, the system SHALL set status to Working
when a prompt turn starts, set status to Idle when the turn ends with no
pending permission request, and set status to Awaiting input when the agent
sends a permission request. The idle timer and input-protection guard used
for terminal/hook-driven agents SHALL NOT apply to `acp-updates` transitions;
they are driven directly by ACP session events, not by inferred silence.

An `acp-updates` transition SHALL carry the same consequences a hook-reported
transition of the same status carries. Entering Awaiting input SHALL raise the
desktop notification `desktop-notifications` requires, and a turn ending SHALL
be treated as going idle for the purpose of the message-delivery nudge, on the
terms `mcp-messaging` sets. A status that reaches the agent's presentation
without those consequences SHALL NOT satisfy this requirement: the observable
behavior is the notification and the nudge, not the colour of the dot alone.

A permission request SHALL supply its own prompt text as the transition's
attention message, so the notification names what is being asked. A transition
with no text available SHALL carry none, and the notification SHALL fall back
to its default body.

#### Scenario: Permission request during a turn

- **WHEN** the agent sends a permission request mid-turn
- **THEN** the agent's status becomes Awaiting input immediately, without
  waiting for any idle timeout

#### Scenario: Permission request raises a notification

- **WHEN** a Panel-mode agent that is not the visible agent sends a permission
  request and `desktop_notifications_enabled` is on
- **THEN** a desktop notification is raised for that agent, bodied with the
  permission request's own prompt text

#### Scenario: Turn ends normally

- **WHEN** a prompt turn's `session/update` stream reports a stop reason of
  `completed` with no pending permission request
- **THEN** the agent's status becomes Idle immediately

#### Scenario: A finished turn is a delivery opportunity

- **WHEN** a Panel-mode agent's turn ends with no pending permission request and
  the agent has an undelivered message
- **THEN** the system re-checks messages for that agent, as it does for an
  agent that goes idle on any other route

#### Scenario: ACP session ends unexpectedly

- **WHEN** the ACP connection for a Panel-mode agent ends with an error (see
  `acp-client`'s subprocess exit handling)
- **THEN** the agent's status becomes Error, matching the terminal path's
  non-zero-exit behavior
