# Tasks

## 1. Server lifecycle state

- [x] 1.1 Add `ServerState` to `knot-mcp` — disabled, starting, running with the bound address,
      retrying with attempt/next-delay/error, stopped — with the payloads carried in the variants;
      verify unit tests assert each variant exposes only its own data.
- [x] 1.2 Add the backoff and probe constants (initial delay, multiplier, maximum delay, probe
      interval, probe timeout, consecutive-failure threshold) to `knot-mcp/src/consts.rs`; verify
      `cargo build --workspace` succeeds.
- [x] 1.3 Implement the pure attempt-number-to-delay policy function; verify unit tests cover
      growth from the initial delay, the cap at the maximum, and that the cap is never exceeded
      for a large attempt count.

## 2. Observing the serve task

- [x] 2.1 Add the additive accessor on `McpServer` that lends the serve task's `JoinHandle` for
      the supervisor to await, leaving `start()`, `stop()` and `Drop` behavior unchanged; verify
      the existing `knot-mcp` tests still pass unmodified.
- [x] 2.2 Verify with a test that the lent handle completes when the serve task ends, and
      completes with an error when the task is aborted.

## 3. Health probe

- [x] 3.1 Implement the raw-TCP `/health` probe — connect, write the minimal request, read the
      status line, succeed on `200` — with a bounded timeout on connect, write and read; verify a
      test probes a live `McpServer` successfully and a test against a closed port fails within
      the timeout rather than hanging.
- [x] 3.2 Implement the consecutive-failure counter: a single failure does not trip, the
      configured number of consecutive failures does, and one success resets it; verify unit
      tests cover all three.

## 4. The supervisor loop

- [x] 4.1 Implement `Supervisor` owning the port, catalog, agents-snapshot function and hook
      handler, constructing a fresh `McpServer` per attempt; verify a test restarts the server
      and asserts the same tools are served afterwards.
- [x] 4.2 Implement the `tokio::select!` loop over stop signal, serve handle and probe timer, and
      expose `state()` as a `watch::Receiver`; verify a test subscribing after the server reaches
      running immediately observes running.
- [x] 4.3 Implement restart on an unexpectedly ended serve task; verify tests cover a serve task
      that returns and one that panics, each followed by the server serving again on the same
      port.
- [x] 4.4 Implement restart on the probe's consecutive-failure threshold; verify a test wedges or
      kills the listener and asserts a restart follows.
- [x] 4.5 Implement bind retry with backoff against the configured port only; verify tests cover
      a busy port producing retrying states with growing delays, recovery once the port is
      released, and that no attempt ever binds a different port.
- [x] 4.6 Implement the stop path so an intentional stop exits before a task exit can be read as
      a failure; verify tests assert that stopping yields the stopped state, issues no restart, no
      probe, and releases the port.
- [x] 4.7 Verify the backoff counter resets only on reaching running *and* passing a probe, so a
      start-then-die loop backs off rather than hot-looping; cover it with a test.
- [x] 4.8 Verify `make lint` and `make size-check` pass for `knot-mcp` and that its `mod.rs`/
      `lib.rs` declare and re-export only.

## 5. Wiring into the app

- [ ] 5.1 Replace the single `server.start().await` in `app_bootstrap::start_mcp_server` with the
      supervisor, keeping the existing thread, runtime and oneshot stop signal; verify the app
      builds and an existing MCP integration test still reaches the server.
- [ ] 5.2 Mirror the watched state into an `Arc<Mutex<ServerState>>` the UI reads, and honour the
      disabled case without starting supervision; verify a test asserts a disabled server binds
      nothing and supervises nothing.
- [ ] 5.3 Push a failure marker into a shared single-slot queue on the transition *into* a
      failing state only; verify unit tests assert one marker per failure episode across several
      retries, and a further marker after a recovery and a second failure.

## 6. Settings pane state row

- [ ] 6.1 Add the localization keys for each state, the retrying attempt and error text, and the
      disabled and stopped text to `crates/knot-core/locales/en.yml`; verify the l10n key tests
      resolve each key (touch `knot-core` first so the catalog is not read from a stale build
      artifact).
- [ ] 6.2 Render the read-only state row in the MCP settings pane, showing the address when
      running, the attempt number and last error when retrying, and the bare state otherwise;
      verify tests drive all five states and assert the row shows nothing belonging to another
      state.
- [ ] 6.3 Add the MCP tab's poll so the row follows a state change while the window stays open,
      notifying only when the state differs from the one last drawn; verify a test asserts no
      repaint is requested for an unchanged state.

## 7. Failure notification

- [ ] 7.1 Drain the failure slot from a window's existing repaint poll with an atomic take, and
      raise a `SystemNotification` identifying the MCP server; verify a test with two windows
      asserts exactly one raises it.
- [ ] 7.2 Gate the notification on `desktop_notifications_enabled`, and make its click bring the
      app to the front without attempting an agent selection; verify tests cover the setting off
      (no notification, state row unchanged in its reporting) and the click path.

## 8. Verification and close-out

- [ ] 8.1 Run the full gate — `make` — and verify `fmt-check`, `size-check`, `lint`, `test` and
      `build` all pass on the workspace.
- [ ] 8.2 Manually verify start retry: hold the configured port with another process, launch
      Knot, confirm the settings row reports retrying with a growing delay, release the port, and
      confirm it reaches running and an agent's MCP tool call succeeds.
- [ ] 8.3 Manually verify failure while running: with an agent connected, kill the serving task's
      listener, confirm the row reports the failure, one notification is raised, the server comes
      back on the same port, and the agent's next tool call succeeds without re-registering.
- [ ] 8.4 Manually verify the shutdown path: quit Knot while the server is running and confirm no
      restart is attempted and the port is released; then disable the server in settings and
      confirm the row reports disabled with nothing bound.
- [ ] 8.5 Verify the notification path in a packaged build — `make package` and `Knot.app` — since
      `UNUserNotificationCenter` delivers nothing under `cargo run`.
