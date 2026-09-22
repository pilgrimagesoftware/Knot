# Tasks

## 1. Entry model and formatting

- [ ] 1.1 Add the log constants to `knot-mcp/src/consts.rs` — rotation size cap, retained rolled-
      file count, heartbeat interval, log file name — and verify `cargo build --workspace` succeeds.
- [ ] 1.2 Add the level and subject enums with `Display` (no stringly-typed vocabulary) and the
      `Entry` type carrying timestamp, level, subject and message; verify unit tests assert each
      variant's rendering.
- [ ] 1.3 Implement entry formatting — UTC RFC 3339 timestamp with a sub-second component, then
      level, subject, message, in that order — and verify unit tests cover the field order and
      that an embedded newline is escaped so the entry stays one line.

## 2. The writer task

- [ ] 2.1 Implement the writer task owning the file: create the directory if absent, open for
      append, and seed the byte counter from the existing file's length; verify tests cover
      creating a missing directory and appending to an existing file without truncating it.
- [ ] 2.2 Implement the `Logger` handle — cheap `Clone`, holding an unbounded sender — and verify a
      test asserts sending does not block and that entries reach the file in order.
- [ ] 2.3 Implement rotation: roll before the write that would exceed the cap, shift rolled files
      up, delete beyond the retained count, reopen; verify tests cover the roll at the cap, the
      deletion of the oldest beyond the retained count, and that entries written continuously
      across a rotation appear exactly once and in order across the files.
- [ ] 2.4 Implement the failure episode: report the first failure to stderr, stay silent for
      subsequent failures, clear on a successful write, and keep draining the channel when the
      file cannot be opened at all; verify tests cover one report across repeated failures, silent
      recovery, and an unopenable path leaving the sender side working.
- [ ] 2.5 Verify `make lint` and `make size-check` pass for `knot-mcp` and that its `mod.rs`/
      `lib.rs` declare and re-export only.

## 3. Server vitals

- [ ] 3.1 Add `len()` to `McpSessionManager` for the live session count; verify a test asserts it
      tracks creation, replacement and stale cleanup.
- [ ] 3.2 Add the start `Instant` and the `Arc<AtomicU64>` served-request counter to the server's
      state, incrementing the counter in the JSON-RPC handler; verify a test asserts the count
      after a known number of requests.

## 4. Wiring the log into the server

- [ ] 4.1 Take the log path as a constructor argument on `McpServer`, construct the logger and
      start the writer task with it, and hold the handle; verify a test constructs a server
      against a temporary path and finds the file created.
- [ ] 4.2 Log the lifecycle events — bind attempted with its target, bind succeeded with the bound
      address, bind failed with the error, server stopped; verify tests assert each entry, using a
      port already in use for the failure case.
- [ ] 4.3 Add a log call beside each of the four existing `eprintln!` sites for request in,
      response out with its error flag, `tools/list` size, and `tools/call`; verify tests assert
      the request and response entries carry method and session, and that stderr output is
      unchanged from before.
- [ ] 4.4 Build the redacted `tools/call` message — tool name, top-level argument key names,
      payload byte size — at the same call site as the full stderr message; verify a test calls a
      tool with message text in its arguments and asserts the text is absent from the file while
      the key names and size are present, and that stderr still prints the arguments in full.

## 5. Heartbeat

- [ ] 5.1 Implement the heartbeat task: on each interval log uptime, bound address, live session
      count, and `swap(0)` of the request counter; verify tests cover an idle interval reporting
      zero, and five-then-two requests reporting five then two rather than five then seven.
- [ ] 5.2 Start the task when the server begins serving and abort it in `stop()` alongside the
      serve handle; verify a test asserts no heartbeat entry is written after the server stops.

## 6. Resolving the path in the app

- [ ] 6.1 Resolve the log directory in `app_bootstrap::start_mcp_server` — `~/Library/Logs/Knot/`
      on macOS, the platform state directory elsewhere — and pass the path to the server; verify a
      unit test asserts the macOS path shape without creating it.
- [ ] 6.2 Verify the app still starts and serves when the resolved directory cannot be created,
      writing to stderr alone.

## 7. Verification and close-out

- [ ] 7.1 Run the full gate — `make` — and verify `fmt-check`, `size-check`, `lint`, `test` and
      `build` all pass on the workspace.
- [ ] 7.2 Manually verify with a packaged build — `make package` and `Knot.app` — that
      `~/Library/Logs/Knot/` holds the log, that connecting an agent produces request and response
      entries, and that a tool call's argument values do not appear in the file.
- [ ] 7.3 Manually verify the heartbeat: leave the app idle past two intervals and confirm two
      heartbeat lines with zero requests, then make several tool calls and confirm the next
      heartbeat reports that count and the one after it reports zero.
- [ ] 7.4 Manually verify rotation by lowering the cap in a scratch build, driving enough traffic
      to roll several times, and confirming the retained file count and that no entry is lost
      across a roll.
