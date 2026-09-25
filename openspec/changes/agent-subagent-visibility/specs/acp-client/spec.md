# Spec Delta

## MODIFIED Requirements

### Requirement: Streaming session updates
The system SHALL deliver `session/update` notifications to the caller as an
ordered stream distinguishing at least: text deltas, tool-call start/update/
result, diffs, and turn-end (with a stop reason). Text deltas for the same
message SHALL be deliverable in arrival order without the caller needing to
re-request the message.

A tool-call start and a tool-call update SHALL carry two things the client
currently discards, when the agent sends them: the call's raw input, and the
notification's metadata envelope. Both SHALL be delivered unparsed and
unmodified.

The protocol's own `kind` is an icon hint and its `title` is prose written for
a human, so neither identifies what tool ran. Adapters therefore identify a
call out of band — in the metadata envelope, and in vendor fields alongside it
— and a client that cannot see those fields cannot recognize a particular tool
call at all. Carrying them is what lets a caller recognize one, such as a
delegation, rather than guess from a label it would also have to keep in step
with the adapter's translations.

A tool call carrying neither field SHALL be delivered with neither, and an
absent field SHALL be distinguishable from an empty one.

#### Scenario: Turn ends mid tool call
- **WHEN** the agent reports a turn-end update while a tool call is still
  open
- **THEN** the system SHALL surface the tool call's last known state alongside
  the turn-end event rather than dropping it

#### Scenario: Raw input reaches the caller
- **WHEN** the agent sends a tool-call start carrying raw input
- **THEN** the caller receives that raw input unmodified alongside the call's
  kind, title and status

#### Scenario: The metadata envelope reaches the caller
- **WHEN** the agent sends a tool-call start carrying a metadata envelope
- **THEN** the caller receives that envelope unmodified, including any
  vendor-specific keys within it

#### Scenario: A tool call without either field
- **WHEN** the agent sends a tool-call start carrying no raw input and no
  metadata
- **THEN** the caller receives the call with neither, distinguishably from a
  call whose raw input or metadata was empty

#### Scenario: An update does not blank what the start carried
- **WHEN** a tool-call update arrives carrying only a status change
- **THEN** the raw input and metadata the caller already holds for that call
  are left intact
