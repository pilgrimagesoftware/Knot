## 1. `knot-terminal`: grid parsing layer

- [ ] 1.1 Add `alacritty_terminal` as a dependency of `knot-terminal` and
      pin an exact version. Verify: `cargo build -p knot-terminal`
      succeeds.
- [ ] 1.2 A `Grid`-owning type wrapping `alacritty_terminal::Term` plus an
      `EventListener` impl that records title/bell/clipboard events for
      later consumption (a plain queue or callback, matching
      `knot-activity::EventSink`'s existing style). Verify: unit test
      feeds known ANSI byte sequences (SGR color codes, cursor movement,
      a title-setting OSC) and asserts the resulting grid cells/cursor
      position/recorded events match expectations.
- [ ] 1.3 Wire PTY output bytes (already captured by `TerminalSession`)
      into the grid type's `Term::input`, replacing/extending whatever
      currently only counts bytes for activity detection. Verify: an
      integration test spawns a real PTY session running a command with
      known output (e.g. `printf`) and asserts the grid reflects it.
- [ ] 1.4 Resize: a method that resizes both the `Term`'s grid dimensions
      and the underlying PTY (`SIGWINCH` via the existing PTY resize
      call, if `knot-terminal` doesn't already expose one - add it if
      not). Verify: unit test resizes a running session and asserts both
      the grid's row/column count and a subsequent `stty size` in the
      PTY reflect the new size.

## 2. GPUI grid rendering in `WorkspaceWindow`

- [ ] 2.1 `TerminalGridElement`: a GPUI `Element` that reads a grid
      handle's visible rows and paints each cell's character with its
      foreground/background/attributes (bold, italic, underline) using
      GPUI's text layout, plus a cursor indicator at the grid's cursor
      position. Verify: a standalone window rendering a static/fixture
      grid (not a live PTY) visually matches expected cell contents
      (manual check - GPUI element rendering has no meaningful headless
      test in this codebase's existing patterns).
- [ ] 2.2 Session lifecycle in `WorkspaceWindow`: on agent selection (or
      workspace open with a pre-selected agent), spawn a session via
      `SessionPlan`/`SessionConfig` if none exists yet, and show its
      grid. Verify: selecting a non-shell agent shows its shell prompt in
      the content pane.
- [ ] 2.3 Detach (hide, don't destroy) the previously-selected agent's
      grid on selection change; keep the session alive so switching back
      doesn't lose scrollback. Verify: typing in agent A, switching to
      agent B, switching back to A shows A's prior output still present.
- [ ] 2.4 Tear down a session when its agent is removed or restarted
      (`agent-lifecycle`'s restart operation gets a fresh session, not a
      reused one). Verify: removing an agent frees its session/grid; a
      unit test on the session-registry type covers this without a real
      subprocess where possible.
- [ ] 2.5 Delete `Shell`, the `OutputBuffer`/`OUTPUT_POLL_INTERVAL`/
      `poll_outputs` scaffold, and any now-unused raw-byte-display code
      now that `WorkspaceWindow` spawns real sessions - this was dead
      code (unreachable from `main`), not a behavior change. Verify:
      `cargo build --workspace` and `cargo test --workspace` still pass
      with it removed; `grep` confirms no remaining references.
- [ ] 2.6 Window resize calls the grid's resize (task 1.4) so the PTY and
      rendered grid track the content pane's size. Verify: manual check
      resizing the window reflows a running program's output (e.g.
      `htop`'s layout adapts).

## 3. Input dispatch

- [ ] 3.1 Keyboard: translate GPUI key events (including modifier
      combinations) from the focused grid element to the bytes/escape
      sequences the PTY expects (printable text, control codes, arrow/
      function keys). Verify: table-driven unit tests cover representative
      keys against expected byte sequences; manual check typed commands
      execute and Ctrl+C/Ctrl+D behave correctly.
- [ ] 3.2 Mouse: click/drag/scroll events translated and written to the
      PTY when the running program has enabled mouse reporting (readable
      from `Term`'s mode flags), falling back to the grid's own
      scrollback/selection otherwise. Verify: manual check a mouse-aware
      TUI (e.g. `htop`) receives clicks, and scrolling a plain shell
      prompt scrolls the view.
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
