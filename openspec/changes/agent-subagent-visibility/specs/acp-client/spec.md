# Spec Delta

## MODIFIED Requirements

### Requirement: Streaming session updates
The system SHALL deliver `session/update` notifications to the caller as an
ordered stream distinguishing at least: text deltas, tool-call start/update/
result, diffs, and turn-end (with a stop reason). Text deltas for the same
message SHALL be deliverable in arrival order without the caller needing to
re-request the message.

A tool-call start and a tool-call update SHALL carry the call's raw input
through to the caller when the agent sends one, unparsed and unmodified. The
protocol's `kind` is an icon hint and its `title` is prose; neither identifies
what tool ran, so the raw input is the only field by which a caller can
recognize a particular tool call — such as a delegation — rather than guess
from a label. A tool call carrying no raw input SHALL be delivered with none,
which SHALL be distinguishable from an empty one.

#### Scenario: Turn ends mid tool call
- **WHEN** the agent reports a turn-end update while a tool call is still
  open
- **THEN** the system SHALL surface the tool call's last known state alongside
  the turn-end event rather than dropping it

#### Scenario: Raw input reaches the caller
- **WHEN** the agent sends a tool-call start carrying raw input
- **THEN** the caller receives that raw input unmodified alongside the call's
  kind, title and status

#### Scenario: A tool call without raw input
- **WHEN** the agent sends a tool-call start carrying no raw input
- **THEN** the caller receives the call with no raw input, distinguishably from
  a call whose raw input was empty

#### Scenario: An update does not blank the start's raw input
- **WHEN** a tool-call update arrives carrying only a status change
- **THEN** the raw input the caller already holds for that call is left intact
