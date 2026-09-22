# Design

## Context

See `proposal.md` — Why. What exists today:

- Four `eprintln!` sites carry everything the MCP server says: two in `server.rs::mcp_rpc`
  (request in, response out, both prefixed with the session id), two in `rpc.rs` (`tools/list`
  size and `tools/call` with its full arguments). `app_bootstrap` prints a fifth line when
  `start()` fails.
- `knot-mcp` depends on tokio, axum, serde, parking_lot, uuid and `knot-agents`. It does **not**
  depend on `knot-core`, so it has neither `l10n` nor `StorePaths`, and this change does not add
  that edge.
- `StorePaths` derives settings-document locations under the preferences and application-data
  directories. The `settings-persistence` contract describes that directory as holding one
  document per durable collection — a log file does not belong there.
- `time` (with `formatting`) and `directories` are already workspace dependencies.
- `McpSessionManager` holds sessions behind a `parking_lot::Mutex` but exposes no count.
- The request path is async, inside axum handlers, on the MCP thread's tokio runtime.

## Goals / Non-Goals

**Goals:**

- Nothing on the request path touches the filesystem.
- The logger is testable without a server: formatting, rotation and the failure-episode rule are
  all exercisable against a temporary directory.
- `knot-mcp` holds no policy about *where* logs live.

**Non-Goals:**

- Beyond `proposal.md`'s non-goals: no structured/JSON log format. The consumer is a person with
  `grep`, and a line-oriented format serves that better than one that reads well only through a
  tool nobody has installed.

## Decisions

### The log path is a constructor argument, not something `knot-mcp` derives

`McpServer` (and the logger it owns) takes the log file's path. `crates/knot` resolves it: on
macOS `~/Library/Logs/Knot/`, which is the platform convention and where Console.app looks;
elsewhere the platform's state directory via `directories`. Tests pass a `tempfile` path.

*Alternative — derive it in `knot-mcp` from `ProjectDirs`:* puts path policy in a crate that has
no other reason to know about the application's directories, and makes every test depend on the
real user's home.

*Alternative — extend `StorePaths` with a `log()` method:* the application-data directory is
specified as holding settings documents, and `~/Library/Application Support` is the wrong place
for a log on macOS regardless. `ProjectDirs` offers no logs directory, so the macOS path is
composed from the home directory directly.

### A writer task behind a channel, not a shared file handle

The logger is a cheap `Clone` handle holding an `mpsc::UnboundedSender<Entry>`. A single writer
task owns the file, the byte counter and the rotation. Handlers format an entry and send it;
sending does not block, does not lock, and does not touch the filesystem.

*Alternative — an `Arc<Mutex<File>>` written inline:* every request would then take a lock and
issue a syscall on the async runtime, and two concurrent requests would contend for it. The
project's standing rule against I/O on hot paths is about the render thread, but the reasoning
transfers: the request path should not wait on a disk.

Serializing through one task also makes rotation safe for free — there is exactly one writer, so
there is no window where another thread writes into a file that is being rolled.

The channel is unbounded deliberately: a bounded channel would have to choose between blocking a
request handler and dropping entries, and the volume here — a handful of lines per request — does
not justify either. If the writer falls behind, memory grows until it catches up, which for a
local diagnostics log is the right failure.

### Entries are formatted at the call site, timestamped at the call site

An entry carries its timestamp from the moment the event happened, not from when the writer got
to it, so a backlog cannot reorder the record relative to reality. `time`'s RFC 3339 formatting
with a UTC offset gives the timestamp; the level and subject are small enums with a `Display`,
per the workspace's rule against stringly-typed vocabularies.

### Rotation by a byte counter the writer maintains

The writer tracks bytes written since the file was opened, seeded from the existing file's length
on open. When a write would take it past the cap, it rolls first: the active file is renamed to
`.1`, existing `.1`/`.2` shift up, anything past the retained count is deleted, a new active file
is opened, and only then is the entry written. Rolling before rather than after the write is what
makes "rotation loses nothing and interleaves nothing" true.

*Alternative — `stat` the file before each write:* a syscall per entry to learn something the
writer already knows.

### Redaction happens where the entry is built

The `tools/call` site builds two different messages: the full-argument one for stderr, which is
exactly the line printed today, and the redacted one — tool name, top-level argument key names,
payload byte size — for the log. Keeping both at the one call site means the difference between
the destinations is visible in the code that creates it, rather than being a filter somewhere
downstream that a future caller could bypass by logging through a different path.

### The heartbeat is a task, and its counter is atomic

A `tokio::spawn`ed loop on the interval, holding the logger, the start `Instant`, the session
manager and an `Arc<AtomicU64>` request counter that `mcp_rpc` increments. Each tick reads the
counter with `swap(0)`, which both reads the interval's total and resets it in one operation, so
a request arriving mid-tick is counted in exactly one interval.

The task's `JoinHandle` is stored beside the serve handle and aborted by `stop()`, which is what
makes heartbeats stop with the server. `McpSessionManager` gains a `len()` for the session count.

### Failure is an episode, tracked in the writer

The writer holds a flag: on the first failure it reports to stderr and sets the flag; subsequent
failures while the flag is set are silent; a successful write clears it. Same shape as the
notification rule in `supervise-mcp-server`, and the same reason — a recurring failure should
produce one line, not a flood.

When the file cannot be opened at all, the writer task still runs and drains the channel,
discarding entries after its one report. That keeps the sender side identical whether logging
works or not, so no call site needs to know.

## Risks / Trade-offs

- **The log records that a tool was called, and by shape what with** → Even redacted, argument
  *key names* and a call sequence say something about what the user is doing. This is a local
  file in the user's own Logs directory, readable only by them, and it is the minimum that makes
  the log useful for the problem it exists to solve. The values — the part that carries content —
  stay out.
- **An unbounded channel can grow** → Only if the writer is blocked on a disk that is not
  responding, in which case memory growth is the least of the situation's problems. Bounded with
  either alternative behavior (block, or drop) being worse for a diagnostics log.
- **A heartbeat every interval makes an idle server the log's main author** → That is the point,
  and it is what the size cap is for. At one line per interval the rotation cap is reached in
  weeks of idling, not hours.
- **Rotation races a concurrent reader** → Someone tailing the file sees it renamed under them.
  Standard for a rotating log and not worth solving; `tail -F` follows by name.
- **Two destinations can drift** → The redaction is a deliberate, specified difference. Any other
  divergence would be a defect, which is why both messages are built at one site.
- **Interaction with `supervise-mcp-server`** → The two changes touch `McpServer`'s start and
  stop paths and `consts.rs`. Whichever lands second reconciles: the supervisor's state
  transitions become log entries, and the logger's lifecycle events move behind them. Neither
  change's specification depends on the other's, and either can land alone.

## Migration Plan

Additive. No persisted data changes shape, no endpoint changes, no agent-visible behavior
changes. An existing installation gains a log directory on next launch. Rollback is reverting the
change; the log files left behind are inert and can be deleted by hand.
