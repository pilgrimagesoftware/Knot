# Proposal

## Why

Everything the MCP server reports about itself goes to stderr: each JSON-RPC method in and out,
the tool catalog size, each tool call, and the one line printed when a bind fails. A packaged
`Knot.app` has no stderr anyone reads, so in the configuration users actually run, that output
goes nowhere. When an agent reports that a Knot tool did not work, there is no record of
whether the server received the call, what it answered, or whether it was even running.

A heartbeat closes the other half of the gap. A silent log is ambiguous — a server that is idle
and a server that is dead look identical — and a regular line that carries the server's vitals
makes the difference legible, in the log and in hindsight.

## What Changes

- Write the MCP server's diagnostics to a log file, in addition to stderr. stderr keeps exactly
  what it prints today, so `cargo run` is unchanged; the file is what a packaged build leaves
  behind.
- Log lines are one per entry, timestamped in UTC, with a level and a short subject, so a line
  can be read on its own and the file can be grepped.
- Record in the log what is printed today — JSON-RPC method in and out with its session,
  `tools/list` size, `tools/call` by name — plus the server's own lifecycle: bind attempted,
  bound to an address, bind failed with its error, stopped.
- **Redact tool-call argument values.** The server prints full `tools/call` arguments to stderr
  today; those arguments carry prompt text, message bodies and file paths. stderr is ephemeral,
  a log file is not, so the file records each call's tool name, its argument keys, and the
  payload's size — not the values. This is a deliberate divergence from the stderr line rather
  than an oversight; it is the one place where the two outputs differ.
- Write a heartbeat line on a fixed interval while the server is serving, carrying uptime, the
  bound address, the number of live MCP sessions, and the number of requests served since the
  previous heartbeat.
- Rotate the log at a size cap, keeping a small fixed number of rolled files and deleting the
  rest, so the file cannot grow without bound across long-running sessions or many launches.
- Keep the server running when logging fails. An unopenable directory or a failed write degrades
  to stderr only and is reported once, never repeatedly and never fatally.

Non-goals, stated so the boundary is explicit:

- No general logging facility. This log belongs to the MCP server. The 44 `eprintln!` calls in
  `crates/knot` and the 8 in `knot-acp` are untouched, and no `tracing` adoption is implied.
- No configurable verbosity, and no setting for the log path, the rotation size, or the
  heartbeat interval. All are constants.
- No log viewer in the app, and no "reveal log" control.
- No change to what the server serves. Endpoints, JSON-RPC lifecycle, tool dispatch and the
  status endpoint are untouched.
- No localization of log lines. The log is a diagnostic artifact read by whoever is debugging,
  not user-facing copy.

## Capabilities

### New Capabilities

None. Logging is behavior of a server that already has a contract.

### Modified Capabilities

- `mcp-server`: gains requirements for a rotating diagnostics log, the content and shape of its
  entries, the redaction of tool-call argument values, the periodic heartbeat, and the rule that
  a logging failure never stops the server.

## Impact

- **`knot-mcp`**: a logger owning the file, the writer task and the rotation; a heartbeat task;
  the entry formatter; the log path taken as a constructor argument rather than derived, so the
  crate holds no path policy and tests write to a temporary directory. `consts.rs` gains the
  rotation size, the retained-file count, and the heartbeat interval. The four existing
  `eprintln!` sites gain a log call beside them.
- **`McpServer` / `AppState`**: a start instant, a served-request counter, and a live-session
  count accessor on `McpSessionManager`, so the heartbeat has vitals to report.
- **`crates/knot`**: `app_bootstrap::start_mcp_server` resolves the log directory — on macOS
  `~/Library/Logs/Knot/`, the platform's state directory elsewhere — and passes the path in.
- **Relationship to `supervise-mcp-server`**: independent. That change adds lifecycle states;
  this one adds a log. Whichever lands second writes the other's events through the facility the
  first established. Neither blocks the other.
