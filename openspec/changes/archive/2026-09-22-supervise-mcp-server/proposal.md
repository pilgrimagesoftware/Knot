# Proposal

## Why

The MCP server is how every agent reaches Knot — messaging, status, registration, repo and
worktree tools. It is started once, on a dedicated thread, and never watched again. If the
serve task ends — a panic inside it, the listener going away — the port goes quiet for the
rest of the app's life: `axum::serve`'s result is discarded, the thread's `tokio::select!`
keeps waiting on a stop signal that has not arrived, and nothing notices. Agents then fail
every tool call against a server the app still believes is running, and the only evidence is
a line on stderr that a `.app` bundle never shows anyone.

The same blind spot covers startup: a bind failure prints and returns, ending the MCP
subsystem for the session.

## What Changes

- Supervise the running server: when the serve task ends without Knot having asked it to stop,
  restart it. This is the change's purpose — the failure that happens after a successful start.
- Add a liveness probe against the server's own `/health` endpoint. A task can keep running
  while the server stops answering, and only a probe distinguishes "serving" from "alive but
  useless". Consecutive probe failures are treated the same as the task ending.
- Retry the initial bind rather than giving up on it, with exponential backoff, capped, against
  the configured port indefinitely. The MCP URL handed to each agent is built from
  `mcp_server_port` at launch time, so recovering on a different port would strand every agent
  already running: the right answer to a busy port is to keep asking for it.
- Give the server an observable lifecycle state — starting, running, retrying, failed — that
  the rest of the app can watch, rather than a `start()` that either returned or did not.
- Show that state in the MCP settings pane: the current state, the bound address when running,
  the attempt count while retrying, and the last error.
- Raise a desktop notification when supervision cannot keep the server up, so a dead server is
  noticed without opening settings.

Non-goals, stated so the boundary is explicit:

- No port fallback. The configured port is the only port, for the reason above.
- No manual restart control. Supervision is automatic; adding a button is a separate decision.
- No change to what the server serves. Endpoints, JSON-RPC lifecycle, tool dispatch, and the
  status endpoint are untouched.
- No change to how agents register or to the registered flag, which lives in the agent store
  and already survives a server restart.
- No persistence of server state or of failure history. The state is runtime-only.
- No supervision of anything else Knot spawns — ACP adapters, terminal shells, the discovery
  watcher all keep their current behavior.

## Capabilities

### New Capabilities

None. This supervises a server that already has a contract rather than introducing a
capability of its own.

### Modified Capabilities

- `mcp-server`: gains requirements for a supervised lifecycle — observable state, restart on
  unexpected serve-task exit, restart on repeated health-probe failure, bounded-backoff retry
  of the initial bind against the configured port, and the guarantee that an intentional stop
  is never mistaken for a failure.
- `settings-ui`: the MCP tab requirement gains the server-state row alongside the existing
  toggle, port field, URL and installation-command generator.
- `desktop-notifications`: gains a requirement for the notification raised when the MCP server
  cannot be kept running, which is the first notification in that capability not tied to an
  individual agent.

## Impact

- **`knot-mcp`**: a supervisor that owns the `McpServer` and its restart policy, a public
  lifecycle-state type published over a `tokio::sync::watch` channel, health probing, and the
  backoff parameters in `consts.rs`. `McpServer::stop` must become distinguishable from a
  failure so the supervisor does not fight the app's own shutdown.
- **`crates/knot`**: `app_bootstrap::start_mcp_server` hands the constructed catalog, hook
  handler and agents snapshot to the supervisor instead of calling `start()` once; the
  oneshot stop signal now stops supervision as well as the server. The state watch is bridged
  into GPUI so the settings pane and the notifier can observe it.
- **`crates/knot/src/settings_window/panes/mcp.rs`**: the new state row.
- **`knot-core`**: `l10n` keys for each state, the retry and failure text, and the notification
  title and body.
- **Restart semantics worth noting, not changing**: MCP session ids do not survive a restart,
  but `POST /mcp` already reissues a session when it receives an unknown one, so clients
  self-heal without re-registering.
