# Tasks

## 1. The decision

- [ ] 1.1 Add `FocusTarget` to `composer_focus.rs` and widen
      `showing_composer` to return it, with `has_live_grid` added to
      `SelectedAgentFacts`; verify `make build` passes
- [ ] 1.2 Extend `tests/composer_focus.rs` with the new spec scenarios - a
      shell agent with a grid focuses the terminal, one still starting focuses
      nothing, a deactivated one focuses nothing, a markdown or diagram pane
      focuses nothing, the dashboard focuses nothing, and switching from a
      panel agent to a shell agent is a transition - and verify
      `cargo test -p knot composer_focus` passes

## 2. Acting on it

- [ ] 2.1 Rename `focused_composer` to the latch over `Option<FocusTarget>` in
      `window.rs` and `open.rs`, and widen `sessions.rs`'s reset to match
      either variant; verify `make build` passes
- [ ] 2.2 Make `focus_showing_composer` focus the terminal handle for
      `FocusTarget::Terminal` and the prompt input for `FocusTarget::Composer`,
      keeping the dialog guard ahead of both
- [ ] 2.3 Fill `has_live_grid` in `prepare_frame` from the selected agent's
      session grid, matching `render/content.rs`'s branch

## 3. Gate

- [ ] 3.1 Run `make fmt` then `make` and verify the whole gate passes
- [ ] 3.2 In the running app, select a shell companion from the sidebar and
      type without clicking the pane; verify the text reaches the shell, that
      selecting one whose session is still starting does not strand focus, and
      that clicking a control elsewhere keeps focus there across redraws
