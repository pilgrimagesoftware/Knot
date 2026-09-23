# Design

## Context

See `proposal.md` — Why. The shape of what exists today:

- `app_bootstrap::start_mcp_server` spawns an OS thread, builds a current-thread-owned tokio
  runtime on it, constructs the tool catalog, calls `McpServer::start().await` once, then parks
  in a `tokio::select!` waiting on a oneshot stop signal. A bind error prints and returns.
- `McpServer::start` binds a `TcpListener`, spawns `axum::serve(listener, router)` and keeps the
  `JoinHandle<()>`. The serve future's result is discarded and the handle is never awaited.
  `stop()` aborts the handle; `Drop` calls `stop()`.
- `McpServer::new` takes the catalog, the agents-snapshot closure and (via `with_hook_handler`)
  the hook handler, all behind `Arc`. They are cheap to clone, so a second `McpServer` can be
  built from the same three values.
- The MCP URL an agent connects to is `knot_agent_launch::mcp_url_for_port(settings.mcp_server_port)`,
  baked into that agent's launch command when the agent starts. That is the constraint that
  rules out a port fallback.
- Desktop notifications are raised with `gpui_kit::SystemNotification` from a window's repaint
  poll, draining a shared queue that an off-main-thread producer filled. Notifications must be
  raised on the main thread, which is why that indirection exists.
- `reqwest` is a dev-dependency of `knot-mcp`, not a runtime one.

## Goals / Non-Goals

**Goals:**

- Make "the server stopped working" a state the app holds, not an eprintln nobody reads.
- Keep the supervisor testable without a GPUI app: the restart policy, the state transitions and
  the probe decision must all be exercisable from `cargo test` in `knot-mcp`.
- Never let supervision fight the app's own shutdown.

**Non-Goals:**

- Beyond `proposal.md`'s non-goals: no attempt to preserve MCP session ids across a restart.
  `POST /mcp` already reissues a session when it receives an unknown one, so clients self-heal;
  making ids durable would be work in service of a problem that is already solved.

## Decisions

### A supervisor in `knot-mcp`, owning the server rather than wrapping it

`Supervisor` holds the port, the catalog, the agents-snapshot function and the hook handler, and
constructs a fresh `McpServer` per attempt. It owns the loop; `McpServer` keeps its current
responsibilities unchanged.

*Alternative — supervise from `crates/knot`:* the loop would then live where it cannot be
tested without an app, and the restart policy would be entangled with GPUI. `knot-mcp` already
depends on tokio and owns the server's lifecycle; the policy belongs beside it.

The loop is a single `tokio::select!` over three arms: the stop signal, the serve task's
`JoinHandle`, and the probe timer. That structure is what makes the spec's "an intentional stop
is never restarted" true by construction rather than by a flag — the stop arm exits the loop
before the serve task's exit can be observed as a failure.

### Detect an ended serve task by awaiting its `JoinHandle`

`McpServer` gains an accessor handing the supervisor a `&mut JoinHandle<()>` to select on
(`JoinHandle` is `Unpin` and is itself a future). Its completion — `Ok(())` from a returned
serve future, or `Err` from a panic or an abort — is the failure signal.

*Alternative — a completion channel the serve task sends on:* an extra moving part that says
nothing the handle does not already say, and one that a panic would skip.

This stays additive: `start()`, `stop()` and `Drop` keep their present behavior, so existing
callers and tests are unaffected.

### Probe `/health` over a raw TCP request, not through an HTTP client

The probe opens a `tokio::net::TcpStream` to the bound address, writes a minimal
`GET /health HTTP/1.1` with `Connection: close`, reads the status line, and succeeds on a `200`.
Connect, write and read are each under one timeout.

*Alternative — promote `reqwest` from dev-dependency to dependency:* a full HTTP client, its
hyper stack and its connection pooling, to issue one unauthenticated GET against loopback. The
probe needs a status line, and testing it against a real server in `knot-mcp`'s tests is still
possible with `reqwest` on the dev side.

A probe failure is a signal, not an error to surface: only the configured number of
*consecutive* failures crosses into a restart, so a single dropped connection during a garbage
collection pause does not bounce the server.

### State as a `tokio::sync::watch` channel

`Supervisor::state()` returns a `watch::Receiver<ServerState>`. `watch` gives the spec's
late-subscriber requirement for free — a receiver reads the current value on subscription — and
its send is lossless for the *latest* value, which is the only one an observer of a state
machine needs.

*Alternative — an `Arc<Mutex<ServerState>>` mirror:* a reader cannot tell a changed state from
an unchanged one without polling, and the notification rule needs edges (one notification per
failure episode), not levels.

`ServerState` carries its payloads in the variant — `Running { addr }`, `Retrying { attempt,
next_delay, error }` — so an observer cannot read an address that belongs to a previous state.

### Bridge the state into GPUI by mirroring, and notify through a claimed slot

The supervisor thread is not the main thread, and `SystemNotification` must be raised on it. So
the bootstrap thread watches the state channel and does two things on each change: writes the
new state into an `Arc<Mutex<ServerState>>` the settings pane reads when it renders, and — on
the transition *into* a failing state, not on each retry — puts a failure marker into a shared
single-slot queue.

Any window's existing repaint poll `take()`s that slot; because `take()` is atomic, exactly one
window raises the notification even with several windows open. With no window open the marker
waits in the slot until one polls. That delay is accepted: with no window open there is nothing
to notify onto, and the state row shows the failure the moment settings is opened.

The settings pane's row needs the window to redraw to follow a change while open. The MCP tab
gets a poll of the same shape as the workspace window's repaint poll, notifying only when the
mirrored state differs from the one last drawn.

### Backoff parameters live in `knot-mcp`'s `consts.rs`

Initial delay, multiplier, maximum delay, probe interval, probe timeout, and the consecutive
failure threshold are all constants in one place, per the workspace convention. The policy
function that maps an attempt number to a delay is pure and unit-tested; nothing about the
schedule requires a running server to verify.

## Risks / Trade-offs

- **A restart loop against a permanently held port** → The supervisor retries forever by design,
  but the capped backoff means it settles at one attempt per maximum-delay interval, and the
  state row says exactly what it is doing and why. Giving up would leave the app in a state no
  user action recovers, since there is no manual restart control.
- **A crash loop caused by the server itself** → If the server starts and then dies immediately,
  supervision restarts it at the same cadence. The backoff counter is reset only by reaching
  *running and probed healthy*, not by a bind that succeeds, so a start-then-die loop backs off
  like any other failure instead of hot-looping.
- **A probe that lies** → `/health` answers from the same axum router as everything else, so it
  cannot report healthy while `/mcp` is wedged at the transport layer, but it says nothing about
  a tool handler that hangs. This change detects an unreachable server, not a slow one; that
  boundary is stated rather than implied.
- **Restarting while an agent has a request in flight** → That request fails. It would have
  failed anyway — the restart is a response to the server not working — and MCP clients retry.
  No attempt is made to drain in-flight work, because the failure path has nothing to drain.
- **Notification delayed with no window open** → Accepted above; the alternative is raising
  notifications from a non-main thread, which is not available.
- **The probe's raw HTTP is hand-rolled** → It parses one status line from a server whose
  response shape this repository controls, and it is unit-tested against the real router. It is
  not a general HTTP client and must not grow into one.

## Migration Plan

Additive and internal. No persisted data changes, no endpoint changes, no change to how agents
are launched or registered. An installation whose MCP server never fails behaves exactly as it
does today, except that the settings pane now shows it running. Rollback is reverting the
change.
