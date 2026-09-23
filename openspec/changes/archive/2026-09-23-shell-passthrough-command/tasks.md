# Tasks

## 1. Shell runner in `knot-processes`

- [x] 1.1 Add `SHELL_OUTPUT_LIMIT` and `SHELL_TIMEOUT` to
      `crates/knot-processes/src/consts.rs` beside `DEFAULT_TIMEOUT`, with a
      comment saying why the shell timeout is not `DEFAULT_TIMEOUT`; verify
      `make lint` passes with no unused-constant warning once 1.3 uses them.
- [x] 1.2 Add `ShellStatus` (`Running`, `Exited { code }`, `Signalled`,
      `Cancelled`, `TimedOut`, `FailedToStart { message }`) and `ShellRunState`
      (stdout buffer, stderr buffer, per-stream truncation flags, status) in a
      new `crates/knot-processes/src/shell/state.rs`; verify unit tests cover
      appending past `SHELL_OUTPUT_LIMIT` — the buffer stops at the limit,
      keeps the head, and sets the truncated flag.
- [x] 1.3 Implement `spawn` in `crates/knot-processes/src/shell/run.rs`:
      `$SHELL -lc <command>` falling back to `/bin/sh -c`, cwd from the caller,
      stdin closed, both pipes drained on worker threads into `ShellRunState`,
      child placed in its own process group; verify a test running `echo hi`
      in a temp dir reports `Exited { code: 0 }` with `hi` on stdout, and one
      running `pwd` reports that temp dir.
- [x] 1.4 Implement the poll loop's timeout kill and a `cancel()` that signals
      the process group; verify tests that a command sleeping past a short
      injected timeout ends `TimedOut`, that `cancel()` ends it `Cancelled`,
      and that in both cases output captured before termination survives.
- [x] 1.5 Expose a `ShellRun` handle carrying `Arc<Mutex<ShellRunState>>`, an
      `Arc<AtomicBool>` dirty flag set on every append, and a clearing
      `take_dirty()`; verify a test that appends set the flag and
      `take_dirty()` clears it exactly once.
- [x] 1.6 Add `ShellError` variants to `crates/knot-processes/src/error.rs` for
      a shell that cannot be launched, re-exported through the crate `Result`;
      verify spawning with a bogus shell path yields `FailedToStart` with the
      OS message rather than a panic.
- [x] 1.7 Declare the `shell` module in `crates/knot-processes/src/lib.rs` with
      re-exports only — no implementation in a `mod.rs`; verify `make
      size-check` and `make test` pass for the crate.

## 2. Trigger recognition

- [x] 2.1 Add shell-command recognition to
      `crates/knot/src/panel_commands/token.rs`: a buffer whose byte 0 is `!`
      followed by at least one non-whitespace character, returning the command
      text; verify unit tests cover `!ls -la`, ` !text` (not a command), `!`
      and `!   ` (neither), a multi-line `!` buffer (whole remainder is the
      command), and that `/` and `@` recognition is unchanged.
- [x] 2.2 Add localization keys to `knot-core` for the shell entry's chrome
      (command, folder, running, exit status, cancelled, timed out, truncated,
      failed to start, pending, shared) and the composer's shell-line marker;
      verify the l10n key-resolution tests pass, touching `knot-core` first
      per the stale-artifact note in the project memory.

## 3. Panel state and rendering

- [x] 3.1 Add `ShellCard { id, command, cwd, stdout, stderr, status, shared }`
      and a `PanelMessage::Shell(ShellCard)` variant in
      `crates/knot/src/panel_state/message.rs`, plus a `push_shell_command`
      helper on `PanelState`; verify existing panel-state tests still compile
      and a new test asserts the pushed card's initial `Running` status.
- [x] 3.2 Render the variant in `crates/knot/src/panel_view/message.rs` and
      handle it in `crates/knot/src/panel_view/rows.rs`: command line, folder,
      monospaced output with stderr distinguished, status, and the truncation
      note; verify the card is visually distinct from a user message, an
      assistant message and a tool call, and that row-height accounting for a
      multi-line card matches what it renders.
- [x] 3.3 Add the cancel control to a running card and the discard control to
      a card whose result is pending; verify each appears only in its own state
      and that a card shows exactly one of pending / shared / neither.

## 4. Execution wiring

- [x] 4.1 Create `crates/knot/src/workspace_window/panel/shell.rs` holding the
      panel's shell submission path, the `panel_shell_runs` side table keyed by
      run id, and the `panel_pending_shell` per-panel pending list; verify
      `make size-check` passes and no implementation lands in a `mod.rs`.
- [x] 4.2 Branch in `send_panel_prompt`
      (`crates/knot/src/workspace_window/panel/prompt.rs`) on the untrimmed
      buffer before the existing trim: a recognised shell command routes to
      4.1, clears the input, and returns without touching
      `deliver_panel_prompt` or the queue; verify a test that submitting `!x`
      leaves the prompt queue and session state untouched.
- [x] 4.3 Resolve the cwd from the selected agent's `folder` at submission
      time, the same lookup `lookup.rs` uses for the skill registry, and spawn
      the run; verify a test that the run receives the agent's folder and that
      a missing agent record fails the card with `FailedToStart` rather than
      running somewhere else.
- [x] 4.4 Drain dirty `ShellRun` handles into their cards in
      `repaint_poll_tick`'s `if` chain, removing finished runs from the side
      table and adding qualifying results to `panel_pending_shell`; verify the
      clearing `take_dirty()` read is reachable only on the path that repaints,
      and a test that a finished `Cancelled` / `TimedOut` / `FailedToStart` run
      never enters the pending list.
- [x] 4.5 Wire the cancel and discard controls to `ShellRun::cancel()` and to
      removal from `panel_pending_shell`; verify tests that cancelling one of
      two running commands leaves the other running, and that a discarded
      result is not attached to a later prompt.

## 5. Hand-off to the agent

- [x] 5.1 Drain `panel_pending_shell` in `send_panel_prompt` alongside
      `panel_pending_context`, appending each result as a block in submission
      order and marking each card shared; verify a test that the delivered text
      carries both commands and their output in order, and that a second prompt
      carries neither.
- [x] 5.2 Verify by test that a run still `Running` when a prompt is sent stays
      pending and rides the next prompt, and that pending results are scoped to
      their panel — a prompt in another panel carries none of them.
- [x] 5.3 Verify by test that with pending results and no prompt sent, nothing
      reaches the ACP session.

## 6. Input area behaviour

- [x] 6.1 Compute the send control's enabled state from the recognition result
      in the control row: a recognised shell command enables send while a
      permission request is pending and while a response is in progress, and a
      bare `!` enables nothing; verify tests over each combination of
      permission-pending, response-in-progress and buffer shape.
- [x] 6.2 Show the composer's shell-line marker while the buffer is recognised
      as a command, driven by 2.1 and clearing as the buffer is edited; verify
      the buffer text is unchanged by the marker appearing or disappearing.
- [x] 6.3 Verified by hand rather than by test: asserting it needs a live
      `WorkspaceWindow`, which needs a real ACP session. Stop interrupts the
      turn and leaves the running command alone.

## 7. Gate

- [x] 7.1 Run `make` and verify the whole gate passes — `fmt-check`,
      `size-check`, `lint` with `-D warnings`, `test`, `build` — with no
      crate-wide `allow` added and every item-level `allow` carrying a reason.
- [x] 7.2 Run the panel by hand against a real agent and verify the end-to-end
      path: `!ls -la` renders and is marked pending, a following prompt reaches
      the agent carrying the output, `!sleep 60` cancels from its card, and a
      command submitted mid-turn neither interrupts the turn nor enters the
      queue.
