# Tasks

## 1. New Agent shortcut

- [ ] 1.1 Add the `NewAgent` action with a fixed ⌘T binding in `keymap/fixed.rs` and a root-element handler on the workspace window that opens the agent editor; verify with a test that ⌘T is bound and that validation rejects a customization onto ⌘T
- [ ] 1.2 Add File > New Agent… below New Workspace, and verify in `tests/menu_key_equivalents.rs` that it shows ⌘T and in a window test that the action is unavailable with no workspace window focused

## 2. Sidebar key hints

- [ ] 2.1 Spike: confirm the root element receives `ModifiersChangedEvent` with the terminal pane and with the composer focused; record the result in design.md before continuing
- [ ] 2.2 Add `SidebarKeyHints`, built from `Resolved` and rebuilt wherever the keymap is applied; verify with unit tests for the defaults, a rebinding, and the nine-agent cap
- [ ] 2.3 Track the ⌘ hold on the workspace window (modifier listener, key-down cancel, 500 ms timer, deactivation reset); verify with gpui tests that hints turn on after the delay, stay off for a quick ⌘C, stay on when ⌥ is added, and clear on release and on deactivation
- [ ] 2.4 Render the hints on the Dashboard, Pull Requests, first nine agent rows and New agent control, full-width and compact, without changing row sizes; verify with a render test that row bounds are equal with hints on and off

## 3. Gate and manual check

- [ ] 3.1 Run `make` and verify the whole gate passes
- [ ] 3.2 Manually verify in the running app: ⌘T and File > New Agent… open the agent editor, including behind a panel; holding ⌘ shows the sidebar hints at both widths, a quick ⌘C does not flash them, and they clear after ⌘Tab
