# acp-client Specification

## Purpose
Lets Knot drive an ACP-speaking agent subprocess as a structured session
(prompts, streaming updates, tool calls, permission requests) instead of a
character stream, independent of any specific agent type.

## Requirements

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
ordered stream distinguishing at least: text deltas, user message chunks,
tool-call start/update/result, diffs, and turn-end (with a stop reason). Text
deltas for the same message SHALL be deliverable in arrival order without the
caller needing to re-request the message.

User message chunks (`user_message_chunk`) are how an agent replays the
user's side of a conversation during `session/load`. They SHALL be delivered
as their own kind, not dropped as unknown, so a caller can show a restored
conversation's prompts between its replies.

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

#### Scenario: A replayed prompt is delivered as the user's
- **WHEN** the agent sends a `user_message_chunk` update while loading a
  session
- **THEN** the system SHALL deliver it as a user message chunk carrying its
  text

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

### Requirement: Permission requests
When the agent sends a `session/request_permission` request, the system
SHALL surface it to the caller as a distinct event carrying the tool call's
id, the action's human-readable title when the agent provides one, and the
available options, and SHALL block sending the caller's decision back to the
agent until the caller responds.

#### Scenario: Session closed while a permission request is pending
- **WHEN** the caller closes the session before responding to a pending
  permission request
- **THEN** the system SHALL respond to the agent with a decline rather than
  leaving the request unanswered

#### Scenario: The request carries the tool call's title
- **WHEN** the agent's permission request includes a `title` for the tool call
- **THEN** the surfaced event includes that title alongside the call's id

#### Scenario: A request without a title is still surfaced
- **WHEN** the agent's permission request carries no tool call title
- **THEN** the event reports no title, and the request is surfaced and answer
  exactly as before

### Requirement: Subprocess exit and error handling
The system SHALL treat unexpected subprocess exit, a broken stdio pipe, or a
JSON-RPC error response to an in-flight request as session-ending conditions
that are reported to the caller with the exit code or error payload, never
as a panic.

#### Scenario: Subprocess crashes mid-turn
- **WHEN** the subprocess exits while a prompt turn is in progress
- **THEN** the system reports the session as ended with the process exit code,
  and any pending prompt/permission futures resolve with an error

### Requirement: A loaded session keeps the id it was loaded by

A successful `session/load` SHALL identify the session by the `sessionId` its
response carries when it carries one, and otherwise by the `sessionId` the
request named. ACP's `LoadSessionResponse` defines no `sessionId`: a loaded
session continues under the id it was asked for. A response without one SHALL
NOT be treated as a failure.

`codex-acp` 2.0.0 answers `session/load` with its models, modes and config
options and no `sessionId`. Treating that as an error made every Codex load
fall back to a fresh session.

#### Scenario: A load response with no session id

- **WHEN** the caller loads session `thread-7` and the agent's successful
  response has no `sessionId`
- **THEN** the loaded session is identified as `thread-7`

#### Scenario: A load response naming its session

- **WHEN** the agent's successful load response carries a `sessionId`
- **THEN** the loaded session is identified by that id

### Requirement: User message chunks keep their metadata

A `user_message_chunk` SHALL be delivered with its `_meta` envelope, verbatim,
alongside its text, or with none when the agent sent none. Not every replayed
user message was typed by the user. An adapter may say who produced one: Claude
Code's origin is `_claude/origin`. A caller can only tell the two apart if the
metadata reaches it.

#### Scenario: An origin tag reaches the caller

- **WHEN** an agent sends a `user_message_chunk` whose `_meta` carries
  `_claude/origin` with kind `task-notification`
- **THEN** the delivered chunk carries that `_meta`

#### Scenario: A chunk without metadata

- **WHEN** an agent sends a `user_message_chunk` with no `_meta`
- **THEN** the delivered chunk carries its text and no metadata

### Requirement: Permission decisions resolve by option kind

Each permission option SHALL be surfaced with its `kind` (`allow_once`,
`allow_always`, `reject_once`, `reject_always`) when the agent sends a known
one. An unknown or malformed kind SHALL leave that option without a kind,
and SHALL NOT discard the request's other options.

A decision SHALL be answered with the option of its own kind, never by the
options' order or wording:

- Allow SHALL answer with the `allow_once` option.
- Always Allow SHALL answer with the `allow_always` option.
- Deny SHALL answer with the `reject_once` option, else the `reject_always`
  option.
- A direct choice SHALL answer with the chosen option.

A decision whose kind the request does not offer SHALL NOT be answered with
some other option.

When no option carries a kind, Allow SHALL answer with the first option and
Deny with the first option whose id or name reads as a refusal, as before
kinds were read.

#### Scenario: Always Allow listed first

- **WHEN** an agent offers Always Allow, Allow and Reject, in that order,
  and the user allows
- **THEN** the agent receives the Allow option's id, not Always Allow's

#### Scenario: Always Allow chosen

- **WHEN** the user chooses Always Allow on that request
- **THEN** the agent receives the Always Allow option's id

#### Scenario: Deny on a Reject option

- **WHEN** the agent's refusal option is named "Reject" with kind
  `reject_once`, and the user denies
- **THEN** the agent receives that option's id

#### Scenario: An unknown kind

- **WHEN** one option carries a kind this client does not know
- **THEN** the request still offers every option, that one without a kind
