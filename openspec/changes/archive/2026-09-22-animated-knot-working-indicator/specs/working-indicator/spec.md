# Spec Delta

## MODIFIED Requirements

### Requirement: Glanceable working indicator

The system SHALL render a glanceable working/thinking indicator for each agent
on the dashboard's agent card, driven by one shared implementation so every
surface that adopts it SHALL NOT drift from the others.

The indicator SHALL reflect the agent's live status emitted by
`activity-detection` — Working, Idle, Awaiting input, or Error — updating in
real time as that status changes, so the user can tell at a glance what an
agent is doing without opening its card.

The indicator SHALL additionally convey whether the agent is running at all,
so an agent that is not running reads as "not running" rather than as
"running-and-idle".

The indicator SHALL NOT be rendered on the workspace sidebar's agent row.
Beside that row's status dot it says the same thing twice, and the one thing
the dot cannot carry — whether the agent is running at all — the row already
carries by dimming as a whole, per `agent-list-ui`'s requirement that a
stopped agent is distinguishable in the sidebar. The card has no such dimming
and no status dot, which is why it takes the indicator and the row does not.

The indicator SHALL NOT be rendered as the agent conversation's
turn-in-progress row either, and that row SHALL NOT be treated as a surface
that has drifted from this one. The two are deliberately different marks: this
indicator distinguishes four states plus not-running in a dense grid, while the
conversation's row conveys one thing — that a turn is running — and is
specified by `acp-panel-ui`. The conversation briefly used this indicator with
its state pinned to Working, which made four of its five states unreachable
there; that is the coupling this sentence exists to prevent being restored.

#### Scenario: Agent is actively working
- **WHEN** `activity-detection` reports the agent as Working
- **THEN** the indicator shows the agent as actively working

#### Scenario: Agent is idle
- **WHEN** `activity-detection` reports the agent as Idle
- **THEN** the indicator shows the agent as idle and thinking, visually
  distinct from actively working

#### Scenario: Agent is awaiting input
- **WHEN** `activity-detection` reports the agent as Awaiting input
- **THEN** the indicator shows the agent as awaiting input, visually distinct
  from both working and idle

#### Scenario: Agent is in error
- **WHEN** `activity-detection` reports the agent as Error
- **THEN** the indicator shows the agent as error, visually distinct from
  working, idle, and awaiting input

#### Scenario: Agent is not running
- **WHEN** the agent is not running (never started or deactivated)
- **THEN** the indicator shows the agent as not running, visually distinct
  from a running-but-idle agent

#### Scenario: The sidebar row carries no indicator
- **WHEN** a workspace sidebar agent row renders, whatever the agent's state
- **THEN** it shows its status dot and no working indicator

#### Scenario: The conversation carries its own mark
- **WHEN** an agent's turn is active and its conversation is open
- **THEN** the conversation's turn-in-progress row is the mark specified by
  `acp-panel-ui`, not this indicator, and the agent's dashboard card still
  shows this indicator as Working
