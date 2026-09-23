# Spec Delta

## ADDED Requirements

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
