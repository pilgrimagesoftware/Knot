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
- The workspace shortcuts' handlers move from app-wide onto the workspace
  window, so each item is enabled only where its shortcut would do something.

This diverges from the Swift reference. Its View menu carries Toggle Git Panel,
Toggle Sidebar, Detach Workspace, Cycle Workspace and Next/Previous Agent,
which the port does not have as shortcuts.

Non-goals:
- Porting the Swift View menu's items.
- New or changed navigation shortcuts. Open Command Center stays in the Window
  menu.
- A menu for the numbered shortcuts beyond nine.
- Showing the keys in the sidebar, and a New Agent shortcut. Both were planned
  here and moved to the `sidebar-key-hints` change so this one could ship.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-menu`: new requirements for the View menu's navigation items (contents,
  order, key equivalents, enablement, checked state and submenu contents). The
  full-screen requirement is unchanged.
- `keybindings` needs no delta: the navigation shortcuts' behavior does not
  change.

## Impact

- `crates/knot/src/app_bootstrap.rs` (`set_app_menus`): builds the View menu.
- `crates/knot/src/menu_bar.rs` / `workspace_window/menus/menu_bar.rs`: the
  menu-bar snapshot and ownership tracking gain what the View menu depends on
  (sidebar agents, workspace names, view mode, selected agent's mode), so the
  bar is rebuilt when any of these changes.
- `crates/knot/src/keymap/handlers.rs`: only Select workspace N stays global.
- `crates/knot/src/workspace_window`: the shortcut handlers on the root element
  (`shortcuts.rs`), and focus kept on a tracked element (`render/mod.rs`,
  `panel/pane.rs`).
- `crates/knot/src/workspace_manager`: workspace changes refresh the menu bar.
- `knot-core` locale: `menu.view.*` label keys.
