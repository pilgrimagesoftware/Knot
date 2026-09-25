# Proposal

Issue: #476

## Why

The navigation shortcuts added by the `keybindings` change (select an agent,
toggle the Dashboard and Pull Requests panels, focus the agent's input, jump to
the bottom, select a workspace) exist only as keys. Nothing in the menu bar
lists them, so a user cannot find them except in the Keyboard settings pane,
and cannot trigger them with the pointer. The View menu is empty apart from
the Enter Full Screen item macOS adds.

## What Changes

- The View menu gains an item for each workspace navigation shortcut, each
  showing its current key equivalent:
  - Dashboard and Pull Requests, checked while their panel is showing;
  - Focus Agent Input and Jump to Bottom;
  - a Select Agent submenu listing the focused workspace's first nine agents by
    name, with ⌥⌘1 … ⌥⌘9;
  - a Select Workspace submenu listing the first nine workspaces by name, with
    ⌘1 … ⌘9.
- Choosing an item does exactly what its shortcut does.
- A rebound shortcut's new key shows beside its item without a restart, as
  Window > Command Center already does.
- Items that act on a workspace window are disabled when no workspace window
  is focused. Select Workspace stays enabled, because its shortcut works with no
  window open.
- macOS's own Enter Full Screen item stays the only full-screen item.
- Holding ⌘ in a workspace window shows each sidebar control's key beside it:
  the Dashboard and Pull Requests rows, the first nine agent rows, and the New
  agent control. The hints appear after a short hold, so ordinary ⌘ shortcuts
  do not flash them, and they follow rebindings.
- New shortcut **⌘T** for New Agent, from the Swift reference, with a File >
  New Agent… item. The New agent control had no key to show.

This diverges from the Swift reference. Its View menu carries Toggle Git Panel,
Toggle Sidebar, Detach Workspace, Cycle Workspace and Next/Previous Agent,
which the port does not have as shortcuts.

Non-goals:
- Porting the Swift View menu's items.
- New or changed navigation shortcuts. Open Command Center stays in the Window
  menu.
- A menu for the numbered shortcuts beyond nine.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-menu`: new requirements for the View menu's navigation items (contents,
  order, key equivalents, enablement, checked state and submenu contents), and
  for File > New Agent… (⌘T). The full-screen requirement is unchanged.
- `agent-list-ui`: holding ⌘ shows the sidebar controls' shortcuts.
- `keybindings` needs no delta: the navigation shortcuts' behavior does not
  change. ⌘T joins the fixed shortcuts a customization is checked against,
  which that spec already covers.

## Impact

- `crates/knot/src/app_bootstrap.rs` (`set_app_menus`): builds the View menu.
- `crates/knot/src/agent_menu.rs` / `workspace_window/menus/menu_bar.rs`: the
  menu-bar snapshot and ownership tracking gain what the View menu depends on
  (sidebar agents, workspace names, view mode, selected agent's mode), so the
  bar is rebuilt when any of these changes.
- `crates/knot/src/keymap`: a fixed ⌘T binding and a global `NewAgent` handler.
  The View items reuse the existing actions and handlers.
- `crates/knot/src/workspace_window`: ⌘-hold tracking and hint rendering in
  the sidebar (`render/sidebar.rs`, `render/mod.rs`).
- `knot-core` locale: `menu.view.*` label keys.
