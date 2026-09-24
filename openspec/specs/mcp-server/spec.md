# mcp-server Specification

## Purpose
Defines the local MCP HTTP server: its endpoints, JSON-RPC lifecycle, MCP
tool-list and tool-call dispatch, the plain-JSON status endpoint used by
external tooling, and MCP session tracking. Behavior is the intended target for
the Rust port; the transport is an implementation choice (axum/hyper), not part
of the contract.

## Requirements

### Requirement: Local bind and configuration

The system SHALL run the MCP server bound to loopback only (`127.0.0.1`) on a
configurable port, default `8766`. The server SHALL be enabled by default and
MAY be disabled by configuration; when disabled, no agent registration prompts
are scheduled and no port is opened.

#### Scenario: Default bind

- **WHEN** the app starts with default settings
- **THEN** the MCP server listens on `127.0.0.1:8766`

#### Scenario: Disabled by configuration

- **WHEN** the MCP server is disabled in settings
- **THEN** no socket is opened and agents receive no registration prompt

### Requirement: Health and info endpoints

The system SHALL expose `GET /health` returning a success indicator and
`GET /` returning basic server info. These SHALL respond without requiring MCP
initialization.

#### Scenario: Health check

- **WHEN** `GET /health` is requested
- **THEN** the response status is 200 with a body indicating the server is up

### Requirement: JSON-RPC endpoint and lifecycle

The system SHALL accept MCP JSON-RPC 2.0 requests at `POST /mcp` and SHALL
implement `initialize` (returning protocol version, server capabilities
declaring tools, and server info), `notifications/initialized`, `tools/list`,
and `tools/call`. Unknown methods SHALL return a JSON-RPC method-not-found
error. A `GET /mcp` request SHALL open a Server-Sent Events stream.

#### Scenario: Initialize handshake

- **WHEN** a client sends `initialize`
- **THEN** the response contains a protocol version, a `tools` capability, and
  server name/version

#### Scenario: Unknown method

- **WHEN** a client calls a method that is not implemented
- **THEN** the response is a JSON-RPC error with code for method-not-found and
  the request id echoed

### Requirement: Tool-list and tool-call dispatch

`tools/list` SHALL return every tool in the catalog with name, description, and
a JSON input schema. `tools/call` SHALL route to the named tool handler and
return an MCP tool result whose `content` is a single text item; failures
SHALL set `isError` true with a human-readable message rather than a transport
error.

#### Scenario: Tool result shape

- **WHEN** any tool call succeeds
- **THEN** the result has one text content item and `isError` is absent or
  false

#### Scenario: Unknown tool

- **WHEN** `tools/call` names a tool not in the catalog
- **THEN** the result has `isError` true and text naming the unknown tool

### Requirement: Status endpoint

The system SHALL expose `GET /api/v1/agent/status` returning a JSON array with
one entry per agent: agent id, name, folder, state, agent-set status text,
registered flag, agent type, session id when present, and hook metadata when
non-empty. Keys SHALL be stably ordered.

#### Scenario: Status reflects live agents

- **WHEN** two agents exist, one registered with a session id
- **THEN** the array has two entries and the registered agent's entry includes
  its `session_id`

### Requirement: MCP session tracking

The system SHALL create an MCP session per agent on demand, keyed by an opaque
id, tracking created-at and last-activity times. Creating a session for an
agent that already has one SHALL replace the old session. Sessions idle beyond
a timeout (default 1 hour) MAY be reclaimed.

#### Scenario: One session per agent

- **WHEN** a session is created for an agent that already has one
- **THEN** the previous session id is no longer resolvable and the new one is

### Requirement: The server writes a diagnostics log to a file

The MCP server SHALL write its diagnostics to a log file, at a path supplied when the server is
constructed, in addition to writing them to standard error. The two destinations SHALL carry the
same events; they differ only in the redaction this specification requires below.

The system SHALL create the log file's directory if it does not exist, and SHALL append to an
existing log file rather than truncating it, so a restart does not discard the record of what
preceded it.

Log lines SHALL NOT be localized. The log is a diagnostic artifact, not user-facing copy.

#### Scenario: A log file is created and appended to

- **WHEN** the server starts with a log path whose directory does not exist
- **THEN** the directory and file are created, and a subsequent restart appends to the same file
  rather than emptying it

#### Scenario: stderr keeps its output

- **WHEN** the server logs any event
- **THEN** the corresponding line still appears on standard error, unchanged from what the
  server printed before the log existed

### Requirement: Each log entry is one self-describing line

Every entry SHALL occupy exactly one line and SHALL carry, in a fixed order: a UTC timestamp
including the date and a sub-second component, a severity level, a short subject naming what the
entry is about, and the entry's message.

An entry's message SHALL NOT contain a newline: a value that would introduce one SHALL be
escaped so the line stays a line. One entry is therefore always one line, and the file can be
searched line-wise.

#### Scenario: Entry shape

- **WHEN** any event is logged
- **THEN** the line carries a UTC timestamp, a level, a subject and a message, in that order

#### Scenario: An embedded newline does not break a line

- **WHEN** a value that would be logged contains a newline
- **THEN** it is escaped and the entry remains a single line

### Requirement: Request, tool and lifecycle events are logged

The system SHALL log:

- each JSON-RPC request received, with its method and the session it belongs to;
- each JSON-RPC response sent, with its method, its session, and whether it carried an error;
- the number of tools returned by `tools/list`;
- each `tools/call`, by tool name;
- each server lifecycle event: a bind attempt with its target address, a successful bind with the
  bound address, a failed bind with the error, and the server stopping.

#### Scenario: A method round-trip is logged

- **WHEN** the server handles a JSON-RPC request
- **THEN** the log holds one entry for the request naming its method and session, and one for the
  response naming its method, session and whether it was an error

#### Scenario: A bind failure is logged

- **WHEN** the server cannot bind its configured address
- **THEN** the log holds an entry naming the address and the error

#### Scenario: A successful bind is logged

- **WHEN** the server binds successfully
- **THEN** the log holds an entry naming the bound address

### Requirement: Tool-call argument values are not written to the log file

For each `tools/call`, the log file SHALL record the tool's name, the names of its top-level
argument keys, and the size of the argument payload. It SHALL NOT record the argument values.

Tool-call arguments carry prompt text, message bodies and file paths. The log file is durable in
a way the standard-error stream is not, so the file records the shape of a call rather than its
content. Standard error SHALL keep printing the arguments as it does today; this is the only
respect in which the two destinations differ.

#### Scenario: Arguments are described, not reproduced

- **WHEN** a tool is called with arguments containing message text
- **THEN** the log file's entry names the tool and its argument keys and gives the payload size,
  and the message text does not appear in the file

#### Scenario: stderr is unaffected by the redaction

- **WHEN** the same call is made
- **THEN** standard error still prints the arguments in full

### Requirement: A heartbeat records the server's vitals

While the server is serving, the system SHALL write a heartbeat entry on a fixed interval,
carrying: how long the server has been up, the address it is bound to, the number of live MCP
sessions, and the number of requests served since the previous heartbeat.

The heartbeat SHALL be written whether or not anything happened in the interval — an idle server
is exactly the case the heartbeat exists to distinguish from a dead one. The requests-served
figure SHALL reset to zero after each heartbeat, so each entry describes its own interval rather
than the whole run.

Heartbeats SHALL start when the server begins serving and stop when it stops. A server that is
not serving SHALL NOT emit them.

#### Scenario: An idle server still beats

- **WHEN** an interval passes with no requests
- **THEN** a heartbeat entry is written, reporting zero requests served in that interval

#### Scenario: The interval count is per interval

- **WHEN** five requests are served in one interval and two in the next
- **THEN** the first heartbeat reports five and the second reports two

#### Scenario: Heartbeats stop with the server

- **WHEN** the server stops
- **THEN** no further heartbeat entries are written

#### Scenario: Vitals are present

- **WHEN** a heartbeat is written while the server is bound and holding sessions
- **THEN** the entry carries the uptime, the bound address, and the live session count

### Requirement: The log is rotated and bounded

The system SHALL bound the space the log occupies. When the active log file reaches a size cap,
the system SHALL roll it aside and begin a new active file, SHALL retain a fixed number of rolled
files, and SHALL delete any older than that.

Rotation SHALL NOT lose an entry that has been accepted for writing, and SHALL NOT interleave a
rolled file's content with the new active file's.

#### Scenario: Reaching the cap rolls the file

- **WHEN** the active log file reaches the size cap
- **THEN** it is rolled aside and a new active file is started

#### Scenario: Old files are deleted

- **WHEN** rotation produces more rolled files than the retained count
- **THEN** the oldest beyond that count are deleted

#### Scenario: Rotation loses nothing

- **WHEN** entries are written continuously across a rotation
- **THEN** every entry appears exactly once, in order, across the rolled and active files

### Requirement: A logging failure never stops the server

A failure to create the log directory, open the log file, write an entry, or rotate SHALL NOT
fail a request, stop the server, or prevent it from starting. The system SHALL continue with
standard error alone.

Such a failure SHALL be reported once per episode, not once per attempt: a log that cannot be
written must not produce a line of complaint for every entry it could not write. Recovery SHALL
be silent — a later successful write ends the episode without further comment.

#### Scenario: An unwritable log directory does not stop startup

- **WHEN** the log path cannot be created or opened
- **THEN** the server starts and serves normally, writing only to standard error

#### Scenario: A write failure is reported once

- **WHEN** writing fails repeatedly
- **THEN** the failure is reported once for that episode, not once per entry

#### Scenario: Requests still succeed

- **WHEN** logging is failing
- **THEN** JSON-RPC requests are served with the same results as when logging works

### Requirement: Observable server lifecycle state

The MCP server SHALL expose a lifecycle state that the rest of the application can observe
and that changes as the server starts, runs, fails, and recovers. The state SHALL be one of:

- **disabled** — configuration has the server turned off; nothing is bound and nothing is
  supervised.
- **starting** — a bind is in progress.
- **running** — bound and serving, carrying the bound address.
- **retrying** — the last attempt failed, carrying the attempt count, the delay before the next
  attempt, and the error that caused it.
- **stopped** — the application asked the server to stop.

Every transition SHALL be observable by a watcher that subscribed before the transition
occurred, and a watcher that subscribes late SHALL immediately see the current state rather
than wait for the next change.

#### Scenario: State reaches running after a successful bind

- **WHEN** the server binds its configured port successfully
- **THEN** its state becomes running and carries the bound address

#### Scenario: A late watcher sees the current state

- **WHEN** a watcher subscribes after the server has already reached running
- **THEN** it immediately observes running, without waiting for another transition

#### Scenario: Disabled by configuration

- **WHEN** the MCP server is disabled by configuration
- **THEN** its state is disabled, no port is bound, and no supervision runs

### Requirement: An unexpectedly ended serve task is restarted

While the server's state is running, the system SHALL watch the task serving requests. If that
task ends for any reason other than the application asking the server to stop — it returns, it
panics, its listener is closed underneath it — the system SHALL treat it as a failure, tear
down any remaining server resources, and start the server again on the configured port.

A restart SHALL reuse the same tool catalog, hook handler, and agent snapshot source the
server was constructed with; it SHALL NOT require agents to register again.

#### Scenario: The serve task returns

- **WHEN** the serve task ends on its own while the state is running
- **THEN** the state leaves running and the server is started again on the configured port

#### Scenario: The serve task panics

- **WHEN** the serve task ends by panicking
- **THEN** it is treated as a failure like any other, and the server is started again

#### Scenario: A restart preserves the served surface

- **WHEN** the server has restarted after a failure
- **THEN** the same tools, hook handling, and status data are served as before the failure, and
  agents that were registered remain registered

### Requirement: A server that stops answering is restarted

While the state is running, the system SHALL probe the server's own health endpoint on a
recurring interval. A probe SHALL be considered failed when it does not return a success
response within a bounded timeout. After a configured number of consecutive failed probes, the
system SHALL treat the server as failed and restart it exactly as it does an ended serve task.

A single failed probe SHALL NOT trigger a restart, and one successful probe SHALL reset the
consecutive-failure count to zero.

#### Scenario: Repeated probe failures restart the server

- **WHEN** the health probe fails for the configured number of consecutive attempts
- **THEN** the server is torn down and started again

#### Scenario: One failed probe is tolerated

- **WHEN** a single health probe fails and the next succeeds
- **THEN** no restart occurs and the consecutive-failure count returns to zero

#### Scenario: Probing stops outside the running state

- **WHEN** the server's state is starting, retrying, stopped, or disabled
- **THEN** no health probes are issued

### Requirement: Start attempts retry on the configured port with backoff

A failed attempt to bind the server SHALL NOT end supervision. The system SHALL retry, waiting
between attempts for a delay that grows exponentially from an initial value up to a maximum,
and SHALL keep retrying for as long as the server is enabled and has not been asked to stop.

Every attempt SHALL target the port the configuration names. The system SHALL NOT bind a
different or ephemeral port in response to a failure: the URL agents are launched with is built
from the configured port, so a server that recovered elsewhere would be unreachable to every
agent already running.

Between attempts the state SHALL be retrying, carrying the attempt count, the delay before the
next attempt, and the error from the last attempt.

#### Scenario: A busy port is retried

- **WHEN** the configured port is held by another process at start
- **THEN** the state becomes retrying and further attempts are made on that same port

#### Scenario: Recovery when the port is released

- **WHEN** the process holding the configured port exits between two attempts
- **THEN** the next attempt binds successfully and the state becomes running

#### Scenario: Backoff grows and is capped

- **WHEN** attempts continue to fail
- **THEN** the delay between them grows from the initial delay and stops growing at the maximum
  delay, rather than growing without bound

#### Scenario: The port never changes

- **WHEN** any number of attempts fail
- **THEN** no attempt binds a port other than the configured one

### Requirement: An intentional stop is never restarted

When the application asks the server to stop — shutdown, or the server being disabled by
configuration — the system SHALL stop supervision first, so the serve task ending is recognized
as the expected consequence of that request and not as a failure. After an intentional stop the
state SHALL be stopped or disabled, no restart SHALL occur, no health probe SHALL be issued,
and the port SHALL be released.

#### Scenario: Shutdown does not trigger a restart

- **WHEN** the application asks the MCP server to stop
- **THEN** the state becomes stopped, the server does not start again, and the port is released

#### Scenario: Disabling the server does not trigger a restart

- **WHEN** the MCP server is disabled by configuration while running
- **THEN** supervision ends, the state becomes disabled, and no restart occurs
