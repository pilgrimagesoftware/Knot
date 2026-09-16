## 1. `knot-terminal`: grid parsing layer

- [x] 1.1 Add `alacritty_terminal` as a dependency of `knot-terminal` and
      pin an exact version. Verify: `cargo build -p knot-terminal`
      succeeds.
- [x] 1.2 A `Grid`-owning type wrapping `alacritty_terminal::Term` plus an
      `EventListener` impl that records title/bell/clipboard events for
      later consumption (a plain queue or callback, matching
      `knot-activity::EventSink`'s existing style). Verify: unit test
      feeds known ANSI byte sequences (SGR color codes, cursor movement,
      a title-setting OSC) and asserts the resulting grid cells/cursor
      position/recorded events match expectations.
- [x] 1.3 Wire PTY output bytes (already captured by `TerminalSession`)
      into the grid type's `Term::input`, replacing/extending whatever
      currently only counts bytes for activity detection. Verify: an
      integration test spawns a real PTY session running a command with
      known output (e.g. `printf`) and asserts the grid reflects it.
- [x] 1.4 Resize: a method that resizes both the `Term`'s grid dimensions
      and the underlying PTY (`SIGWINCH` via the existing PTY resize
      call, if `knot-terminal` doesn't already expose one - add it if
      not). Verify: unit test resizes a running session and asserts both
      the grid's row/column count and a subsequent `stty size` in the
      PTY reflect the new size.

## 2. GPUI grid rendering in `WorkspaceWindow`

- [x] 2.1 Renders every visible row's cells (foreground/background,
      bold/italic/underline/strikeout, consecutive same-style cells
      merged into one span for reasonable element count) plus a cursor
      (fg/bg swap at the cursor cell) via a plain function
      (`terminal_view::render_grid`) rather than a custom GPUI `Element`
      impl - simpler for a first cut since nothing here needs
      layout/paint-level control; revisit only if profiling shows the
      per-frame div count is a real cost. Verify: `cargo build`/`clippy`
      clean; manual check in the running app (task 5.2).
- [x] 2.2 Session lifecycle in `WorkspaceWindow`: on agent selection (or
      workspace open with a pre-selected agent), spawn a session via
      `SessionPlan`/`SessionConfig` if none exists yet (`ensure_session`),
      and show its grid. Verify: selecting a non-shell agent shows its
      shell prompt in the content pane (manual check, task 5.2).
- [x] 2.3 Detach (hide, don't destroy) the previously-selected agent's
      grid on selection change; keep the session alive so switching back
      doesn't lose scrollback - `ensure_session` only spawns if the
      `sessions` map has no entry yet, and selection change never removes
      an entry. Verify: manual check (task 5.2).
- [ ] 2.4 Tear down a session when its agent is removed or restarted
      (`agent-lifecycle`'s restart operation gets a fresh session, not a
      reused one). `remove_session` + a `Drop` impl (shuts down every
      session when the window closes) exist, but nothing in
      `WorkspaceWindow` yet exposes remove/restart-agent UI to call it
      from - `Shell`'s dead code had `close_agent`/`restart_agent`, but
      porting those UI actions is a `dashboard-view`/agent-management
      concern, not terminal-rendering's. Revisit once that UI exists.
- [x] 2.5 Delete `Shell`, the `OutputBuffer`/`OUTPUT_POLL_INTERVAL`/
      `TerminalModel`/`AgentHeader`/`terminal_model`/`visible_output`
      scaffold now that `WorkspaceWindow` spawns real sessions - this was
      dead code (unreachable from `main`), not a behavior change.
      Verified: `cargo build --workspace`/`test --workspace` pass with it
      removed; `grep` confirms no remaining references.
- [x] 2.6 Window resize calls the grid's resize (task 1.4) so the PTY and
      rendered grid track the content pane's size, via
      `resize_session_to_pane` reading `window.viewport_size()` each
      render and diffing against the grid's current size. Cell dimensions
      are an approximation (not a real glyph measurement) - revisit if
      layout drifts noticeably. Verify: manual check (task 5.2).

## 3. Input dispatch

- [x] 3.1 Keyboard: translate GPUI key events (including modifier
      combinations) from the focused grid element to the bytes/escape
      sequences the PTY expects (printable text, control codes, arrow/
      function keys) - `knot_terminal::key_to_bytes` (framework-agnostic,
      takes a plain `KeyInput`), wired via a focusable pane
      (`WorkspaceWindow::dispatch_key`, click the pane to focus it).
      Verified: 9 table-driven unit tests cover letters, shifted letters,
      Ctrl combos, Enter (sends `\r` not its key_char), arrows, function
      keys, Alt-prefixing, and bare modifier presses; manual check
      pending (task 5.2).
- [x] 3.2 Mouse: click/scroll events translated (`knot_terminal::mouse_to_bytes`,
      5 unit tests covering left/middle/right buttons, press vs. release,
      and wheel up/down) and written to the PTY when the running program
      has enabled SGR mouse reporting (`Grid::sgr_mouse_mode`, checks
      `TermMode::SGR_MOUSE` + `MOUSE_MODE`), via
      `WorkspaceWindow::dispatch_mouse_button`/`dispatch_scroll`. Drag
      isn't sent (no motion-mode tracking yet), and the non-mouse-reporting
      fallback is a no-op rather than driving scrollback/selection - that
      view doesn't exist yet (see task 3.3). Verify: manual check pending
      (task 5.2).
- [ ] 3.3 Text selection: click-drag over the grid selects text (using
      `alacritty_terminal`'s selection support), with copy sending the
      selected text to the OS pasteboard per the terminal-actions spec's
      transform rules (strip ANSI, trim trailing whitespace). Verify: unit
      tests cover the transform functions directly; manual check
      select-and-copy round-trips through the OS pasteboard.

## 4. Action routing

- [ ] 4.1 Title-change events (task 1.2's recorded events) update
      `Agent::terminal_title` through the existing `AgentStore` mutation
      path. Verify: unit test feeds a constructed title event through the
      handler and asserts the agent's `terminal_title` changes; manual
      check a shell `printf '\e]0;title\a'` updates the sidebar.
- [ ] 4.2 Terminal-output activity and process-exit events call
      `Tracker::on_terminal_activity`/`on_process_exit`, replacing the old
      polling loop's calls to the same methods (task 2.5 removes the old
      caller). Verify: existing `knot-activity` tests still pass unchanged
      (its API didn't change) plus one new integration test that feeding
      a spawned session's output triggers a status transition.
- [ ] 4.3 OSC 52 clipboard write requests (task 1.2's recorded events)
      serve the OS pasteboard, applying the same default text transforms
      as task 3.3's copy path. Verify: manual check a program using OSC 52
      to set the clipboard (e.g. `printf '\e]52;c;...\a'`) updates the OS
      pasteboard.

## 5. Final verification

- [ ] 5.1 `cargo fmt --all --check` (or stable `cargo fmt` if nightly is
      unavailable), `cargo clippy --workspace --all-targets -- -D
      warnings`, `cargo test --workspace`, `cargo build --workspace` all
      pass clean.
- [ ] 5.2 Manual verification end-to-end: create an agent, watch its real
      shell prompt render, type a command and see output, switch agents
      and back, restart an agent and confirm a fresh session, remove an
      agent and confirm no leaked session/process, resize the window,
      select and copy text, and paste - all in one pass against a debug
      build.
