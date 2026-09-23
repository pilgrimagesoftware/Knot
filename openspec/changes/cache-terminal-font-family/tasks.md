# Tasks

## 1. The resolution rule as a pure function

- [ ] 1.1 Add `crates/knot/src/workspace_window/terminal_font.rs` with
      `resolve(requested: &str, available: &[String]) -> SharedString`,
      carrying `terminal_font_family`'s rule and its doc comment; declare the
      module in `workspace_window/mod.rs` and verify `make build` passes
- [ ] 1.2 Add unit tests for `resolve` covering an installed name, an
      uninstalled name falling back to JetBrains Mono, and an empty name, and
      verify `cargo test -p knot terminal_font` passes

## 2. The per-window memo

- [ ] 2.1 Add the memo type to `terminal_font.rs` (the name that was asked
      for plus the family it resolved to) with a `refresh` that re-resolves
      only when the requested name differs, and a test asserting that
      resolving the same name repeatedly asks for the available-families list
      exactly once while a changed name asks again - the regression test for
      this change's spec requirement
- [ ] 2.2 Add the `terminal_font` field to `WorkspaceWindow` in
      `window.rs`, initialised wherever the window is constructed, and verify
      `make build` passes

## 3. Moving the call sites onto the memo

- [ ] 3.1 Refresh the memo from `prepare_frame` in `render/mod.rs`, ahead of
      `resize_session_to_pane`
- [ ] 3.2 Replace `terminal_font_family(&self.settings, cx)` with the memo
      read in `terminal_input.rs` (`resize_session_to_pane`, `grid_position`)
      and `render/content.rs` (`render_grid`, the scroll listener)
- [ ] 3.3 Delete `terminal_font_family` from `chrome.rs` and verify
      `rg 'all_font_names' crates/knot/src` reports only the settings window

## 4. Gate

- [ ] 4.1 Run `make fmt` then `make` and verify the whole gate passes
      (fmt-check, size-check, clippy with `-D warnings`, workspace tests,
      build)
- [ ] 4.2 Launch the app, open a shell agent's terminal and type a line, and
      verify the echo keeps up with typing
