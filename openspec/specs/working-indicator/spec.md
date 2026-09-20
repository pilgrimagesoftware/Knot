# working-indicator Specification

## Purpose
Provides a single glanceable working/thinking visual mark per agent that
reflects the agent's live Working / Idle / Awaiting input / Error status —
consuming the status `activity-detection` already emits — so a bench of agents
can be read at a glance without reading a status string, and so a not-running
agent stays visually distinct from a running-but-idle one.

## Requirements

### Requirement: Glanceable working indicator

The system SHALL render a glanceable working/thinking indicator for each agent
wherever that agent's status is already shown — the workspace sidebar's agent
row and the dashboard's agent card — driven by a single shared implementation
so the two surfaces SHALL NOT drift.

The indicator SHALL reflect the agent's live status emitted by
`activity-detection` — Working, Idle, Awaiting input, or Error — updating in
real time as that status changes, so the user can tell at a glance what an
agent is doing without opening its row.

The indicator SHALL be visually distinct from the existing per-agent status
dot: the dot reports what a *running* agent is doing, while the indicator
SHALL additionally convey whether the agent is running at all, so an agent
that is not running SHALL read as "not running" rather than as
"running-and-idle".

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

### Requirement: Animated state change

When an agent's status changes, the indicator SHALL animate to draw the
user's attention, so a Working-to-Idle or Idle-to-Working transition reads
even in peripheral vision.

#### Scenario: Transition while user looks elsewhere
- **WHEN** the agent transitions from Working to Idle
- **THEN** the indicator animates its change so the user notices without
  reading text
