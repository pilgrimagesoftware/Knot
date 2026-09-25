# Proposal

Issue: https://github.com/pilgrimagesoftware/Knot/issues/486

## Why

Switching between agents in the same workspace happens far more often than
switching between workspaces, yet the more frequent action has the harder
chord: ⌥⌘1–⌥⌘9 selects an agent, while ⌘1–⌘9 selects a workspace. The easier
chord should go to the more frequent action.

## What Changes

- The default agent-selection modifier changes from ⌥⌘ to ⌘, so ⌘1–⌘9 selects
  the Nth agent in the focused workspace window.
- The default workspace-selection modifier changes from ⌘ to ⌥⌘, so ⌥⌘1–⌥⌘9
  opens or raises the Nth workspace.
- **BREAKING** (muscle memory only): anyone on the defaults gets the swapped
  chords after upgrading. A modifier the user has customized is kept, because
  only values that differ from the default are stored. No preference
  migration is needed.
- The `keybindings` spec's note on the Swift reference records that ⌘1–⌘9 now
  deliberately diverges from the reference's workspace switcher.
- Everything that displays a select shortcut (Keyboard settings tab, View
  menu items, sidebar hints) reads the resolved keymap, so it follows the new
  defaults without its own change.

Non-goals:

- No change to the other shortcuts' defaults (⌘L, ⌥⌘O, ⌥⌘P, ⌃⌘↓, ⌥⌘0), or
  to ⌘0 opening the workspace manager.
- No change to scope: workspace selection still works from any window, and
  agent selection only in the focused workspace window.
- No option to restore the old layout beyond the existing Keyboard tab
  modifier choices.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `keybindings`: the default modifiers of the two numbered families swap, and
  the requirements' examples and scenarios that name those defaults are
  updated to match.
- `app-menu`: the View menu's Select Agent and Select Workspace submenus
  name the swapped defaults (⌘N for agents, ⌥⌘N for workspaces).

## Impact

- `crates/knot/src/keymap/shortcut.rs`: `Shortcut::default_modifiers` for
  `SelectWorkspace` and `SelectAgent`.
- Tests that assert the old defaults: `crates/knot/src/keymap/tests.rs`,
  `crates/knot/src/tests/keybindings.rs`,
  `crates/knot/src/tests/settings_keyboard.rs`, and
  `crates/knot/src/workspace_window/shortcuts_tests.rs` if it presses chords
  rather than dispatching actions.
- The unstarted `sidebar-key-hints` change (#481) names ⌥⌘N as the agent
  default in its `agent-list-ui` delta and design; this change updates that
  text in place.
- Stored preferences: no format or migration change.
