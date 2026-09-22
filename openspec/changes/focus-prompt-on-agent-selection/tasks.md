# Tasks

## 1. Confirm the dialog guard before building on it

- [ ] 1.1 Determine whether a dialog opened with `window.open_alert_dialog`
      holds focus inside `root_focus`'s subtree — add a temporary trace of
      `self.root_focus.contains_focused(window, cx)` in `prepare_frame`, open a
      confirmation dialog (Remove Agent will do), and read the result; verify by
      observing the value with the dialog open and closed
- [ ] 1.2 If containment is true with a dialog open, the guard in design.md does
      not work — record what it has to become in design.md before writing any
      of section 2, and remove the trace either way

## 2. The focus transition

- [ ] 2.1 Add a pure helper that answers "which agent's composer will this frame
      show", taking the selected agent and the facts the content pane branches
      on — panel-vs-terminal view mode, markdown pane, diagram pane, activation,
      and whether the window is showing its dashboard — and returning
      `Option<Uuid>`; verify with `make build`
- [ ] 2.2 Add `crates/knot/src/tests/composer_focus.rs` (registered in
      `tests/mod.rs`) covering that helper: a Panel-mode activated agent yields
      its id; a Terminal-mode agent, a deactivated agent, an agent with a
      markdown file, an agent with a diagram, and the dashboard view each yield
      `None`; verify with `cargo test -p knot composer_focus`
- [ ] 2.3 Add `focused_composer: Option<Uuid>` to `WorkspaceWindow`, initialised
      `None` in `open.rs`, with a doc comment saying it records the last
      *focused* composer so focus is taken on a transition and never held;
      verify with `make lint`
- [ ] 2.4 In `prepare_frame`, compute the showing id with the 2.1 helper, and
      when it differs from `focused_composer` and is `Some(id)` — and focus is
      nowhere or within `root_focus` — focus that agent's composer via
      `panel_prompt_input(id, window, cx)`, storing the computed value either
      way; place it before the existing root-focus guard; verify with `make lint`
      and by clicking a Panel-mode agent and typing
- [ ] 2.5 Clear the entry for a removed agent in `teardown_session` alongside the
      other per-agent state, per the parallel-maps rule; verify with `make lint`
      and by removing the selected agent

## 3. Check what focus now affects

- [ ] 3.1 Confirm the Agents menu still enables and dispatches with a focused
      composer — the root-focus guard no longer fires, and the menu's handlers
      depend on the composer being the root element's descendant; verify by
      selecting a Panel-mode agent and opening the Agents menu
- [ ] 3.2 Confirm the Edit menu's five items are enabled with the composer
      focused and that ⌘C copies from it, per `app-menu`'s requirement; verify by
      selecting text in the composer and choosing Edit > Copy
- [ ] 3.3 Confirm the terminal pane is unaffected: selecting a Terminal-mode
      agent leaves focus alone, and clicking the pane still focuses it and still
      answers ⌘C with its own copy-selection

## 4. Walk the spec

- [ ] 4.1 Select a Panel-mode agent from the sidebar, from the window's overview,
      and by creating one, and confirm typing goes into the composer each time
      with no intervening click
- [ ] 4.2 Open a workspace window whose restored selection is a Panel-mode agent
      and confirm its composer has focus
- [ ] 4.3 Type a partial message to one agent, switch to a second, switch back,
      and confirm the first agent's composer has focus and still holds its text
- [ ] 4.4 Select an agent, click a config selector, and let the window redraw
      (start a turn so it streams); confirm focus stays where it was put
- [ ] 4.5 Confirm the no-focus cases: dashboard showing, markdown pane open,
      diagram pane open, deactivated agent, Terminal-mode agent
- [ ] 4.6 Run `make` and confirm the whole gate passes — `fmt-check`,
      `size-check`, `clippy -D warnings`, tests, build
