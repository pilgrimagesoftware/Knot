## Purpose

Lets Knot drive an ACP-speaking agent subprocess as a structured session
(prompts, streaming updates, tool calls, permission requests) instead of a
character stream, independent of any specific agent type.

## ADDED Requirements

### Requirement: Transport and framing
The system SHALL communicate with an ACP agent as a spawned subprocess,
sending and receiving JSON-RPC 2.0 messages framed over the subprocess's
stdio. The system SHALL support both individual JSON-RPC messages and
batched arrays on both directions.

#### Scenario: Malformed message from the subprocess
- **WHEN** the subprocess writes a line that is not valid JSON-RPC
- **THEN** the system discards that line, logs it, and keeps the connection
  open rather than tearing down the session

### Requirement: Capability negotiation
On connecting to a subprocess, the system SHALL send an `initialize` request
carrying its own protocol version and capabilities, and SHALL record the
agent's declared protocol version and capabilities from the response before
issuing any other request.

#### Scenario: Unsupported protocol version
- **WHEN** the agent's `initialize` response reports a protocol version the
  client does not support
- **THEN** the system SHALL fail the connection with a typed error instead of
  proceeding to session creation

### Requirement: Session lifecycle
The system SHALL support creating a new session (`session/new`), resuming a
prior session (`session/load`) when the agent's capabilities advertise
support for it, sending a prompt (`session/prompt`), cancelling the active
turn (`session/cancel`), and closing the session. Each session is identified
by the id returned from `session/new` or `session/load`.

#### Scenario: Resume unsupported
- **WHEN** the caller requests `session/load` for an agent whose capabilities
  do not advertise session loading
- **THEN** the system SHALL return a typed "not supported" error without
  sending the request

### Requirement: Streaming session updates
The system SHALL deliver `session/update` notifications to the caller as an
ordered stream distinguishing at least: text deltas, tool-call start/update/
result, diffs, and turn-end (with a stop reason). Text deltas for the same
message SHALL be deliverable in arrival order without the caller needing to
re-request the message.

#### Scenario: Turn ends mid tool call
- **WHEN** the agent reports a turn-end update while a tool call is still
  open
- **THEN** the system SHALL surface the tool call's last known state alongside
  the turn-end event rather than dropping it

### Requirement: Permission requests
When the agent sends a `session/request_permission` request, the system
SHALL surface it to the caller as a distinct event carrying the requested
action and available options, and SHALL block sending the caller's decision
back to the agent until the caller responds.

#### Scenario: Session closed while a permission request is pending
- **WHEN** the caller closes the session before responding to a pending
  permission request
- **THEN** the system SHALL respond to the agent with a decline rather than
  leaving the request unanswered

### Requirement: Subprocess exit and error handling
The system SHALL treat unexpected subprocess exit, a broken stdio pipe, or a
JSON-RPC error response to an in-flight request as session-ending conditions
that are reported to the caller with the exit code or error payload, never
as a panic.

#### Scenario: Subprocess crashes mid-turn
- **WHEN** the subprocess exits while a prompt turn is in progress
- **THEN** the system reports the session as ended with the process exit code,
  and any pending prompt/permission futures resolve with an error
