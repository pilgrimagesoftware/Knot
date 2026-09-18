## MODIFIED Requirements

### Requirement: Activity-tracking presets

The system SHALL assign each agent an activity-tracking set drawn from
`{user-input, terminal-output, acp-updates}`. Shell agents SHALL track none
of these and SHALL be forced to Idle regardless of any status write. An
agent in Panel view mode (ACP-managed) SHALL track only `acp-updates`; the
`user-input` and `terminal-output` trackers SHALL be ignored for it even if
its underlying terminal process still exists. All other agents SHALL track
both `user-input` and `terminal-output`. The tracking set MAY be downgraded
at runtime (for example when hook-based detection takes over, or when an
agent switches into Panel mode); callbacks for a tracker not in the current
set remain wired but are ignored.

#### Scenario: Shell agent never leaves Idle

- **WHEN** a shell agent produces heavy terminal output
- **THEN** its status stays Idle

#### Scenario: Runtime downgrade stops terminal-output transitions

- **WHEN** an agent's tracking set is downgraded to `{user-input}`
- **THEN** subsequent terminal output does not move it to Working, but it still
  cancels a pending idle timer

#### Scenario: Switching to Panel mode stops terminal/keystroke tracking

- **WHEN** an agent switches from Terminal to Panel mode
- **THEN** its tracking set becomes `{acp-updates}` and further terminal
  output or keystrokes in a still-open terminal view no longer change its
  status

## ADDED Requirements

### Requirement: ACP updates drive status for Panel-mode agents

For an agent tracking `acp-updates`, the system SHALL set status to Working
when a prompt turn starts, set status to Idle when the turn ends with no
pending permission request, and set status to Awaiting input when the agent
sends a permission request. The idle timer and input-protection guard used
for terminal/hook-driven agents SHALL NOT apply to `acp-updates` transitions;
they are driven directly by ACP session events, not by inferred silence.

#### Scenario: Permission request during a turn

- **WHEN** the agent sends a permission request mid-turn
- **THEN** the agent's status becomes Awaiting input immediately, without
  waiting for any idle timeout

#### Scenario: Turn ends normally

- **WHEN** a prompt turn's `session/update` stream reports a stop reason of
  `completed` with no pending permission request
- **THEN** the agent's status becomes Idle immediately

#### Scenario: ACP session ends unexpectedly

- **WHEN** the ACP connection for a Panel-mode agent ends with an error (see
  `acp-client`'s subprocess exit handling)
- **THEN** the agent's status becomes Error, matching the terminal path's
  non-zero-exit behavior
