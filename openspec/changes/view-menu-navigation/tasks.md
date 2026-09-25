# Tasks

## 1. Menu bar snapshot

- [ ] 1.1 Rename `AgentMenuSnapshot` / `AgentsMenuState` to `MenuBarSnapshot` / `MenuBarState` and update every caller; verify `make lint` and `make test` pass with no behavior change
- [ ] 1.2 Add the `view` part to the snapshot (workspace-window flag, view mode, selected agent's mode, first nine sidebar agents with the selection, first nine workspaces with the owner's) and fill it in `selected_agent_menu`'s pass over the store; verify with unit tests that selecting an agent, toggling a panel, and adding or renaming an agent or workspace each change the snapshot, and that the untouched case compares equal
- [ ] 1.3 Fill `workspaces` from the store when no workspace window owns the bar; verify with a unit test that the owner-less snapshot lists the workspaces

## 2. View menu

- [ ] 2.1 Add `menu.view.*` label keys to `crates/knot-core/locales/en.yml` (touch `knot-core` afterwards) and register them in `tests/l10n_catalog.rs`; verify the catalog test passes
- [ ] 2.2 Create `crates/knot/src/view_menu.rs` with `view_menu(&MenuBarSnapshot) -> Menu`: items in the spec's order, each on the shortcut's existing action, `disabled` and `checked` from the snapshot; verify with unit tests on the built `Menu` for each enablement and checked-state scenario in the spec
- [ ] 2.3 Build the Select Agent and Select Workspace submenus from the snapshot on `SelectAgentN` / `SelectWorkspaceN`; verify with unit tests for the nine-item cap, the checked item, and the disabled empty submenu
- [ ] 2.4 Replace `Menu::new("View").items([])` in `set_app_menus` with `view_menu(snapshot)`, and assert in `tests/menu_key_equivalents.rs` that every View item shows its default key and that the menu declares no full-screen item

## 3. Keeping the menu current

- [ ] 3.1 Add `refresh_menu_bar_workspaces(cx)` and call it after each workspace manager mutation (create, rename, delete, reorder); verify with a test that renaming a workspace from the manager changes the snapshot's workspace list
- [ ] 3.2 Verify with a test that a rebinding (`apply_and_refresh_menus`) makes the View item show the new chord

## 4. New Agent shortcut

- [ ] 4.1 Add the `NewAgent` action with a fixed ⌘T binding in `keymap/fixed.rs` and a global handler that opens the agent editor in the active workspace window; verify with a test that ⌘T is bound and that validation rejects a customization onto ⌘T
- [ ] 4.2 Add File > New Agent… below New Workspace, disabled when no workspace window owns the bar; verify in `tests/menu_key_equivalents.rs` that it shows ⌘T

## 5. Sidebar key hints

- [ ] 5.1 Spike: confirm the root element receives `ModifiersChangedEvent` with the terminal pane and with the composer focused; record the result in design.md before continuing
- [ ] 5.2 Add `SidebarKeyHints`, built from `Resolved` and rebuilt wherever the keymap is applied; verify with unit tests for the defaults, a rebinding, and the nine-agent cap
- [ ] 5.3 Track the ⌘ hold on the workspace window (modifier listener, key-down cancel, 500 ms timer, deactivation reset); verify with gpui tests that hints turn on after the delay, stay off for a quick ⌘C, stay on when ⌥ is added, and clear on release and on deactivation
- [ ] 5.4 Render the hints on the Dashboard, Pull Requests, first nine agent rows and New agent control, full-width and compact, without changing row sizes; verify with a render test that row bounds are equal with hints on and off

## 6. Gate and manual check

- [ ] 6.1 Run `make` and verify the whole gate passes
- [ ] 6.2 Manually verify in the running app: each View item does what its key does; checkmarks and enablement follow the focused window; the submenus follow agent and workspace changes; exactly one Enter Full Screen item appears and works; ⌘T opens the agent editor; holding ⌘ shows the sidebar hints at both widths, and they clear after ⌘Tab
