# Proposal

## Why

Knot has no keyboard route to its main navigation: moving between workspace
windows, picking an agent, returning to its input, and opening the dashboard or
pull-request panel all take the mouse, which is a barrier for keyboard-only
users (issue [#454](https://github.com/pilgrimagesoftware/Knot/issues/454)).
The shortcuts that do exist are hard-coded, so a chord that clashes with a
user's other tools cannot be moved.

## What Changes

- Add shortcuts that:
  - select workspace 1–9 (default ⌘1–⌘9), opening its window or raising it;
  - select agent 1–9 in the focused workspace window (default ⌥⌘1–⌥⌘9);
  - focus the selected agent's input (default ⌘L);
  - toggle the workspace's Dashboard panel (default ⌥⌘O);
  - toggle the workspace's Pull Requests panel (default ⌥⌘P);
  - jump to the bottom of the selected agent's conversation and resume
    following new output (default ⌃⌘↓);
  - open or raise the Command Center (⌥⌘0; already exists, now customizable).
- Add a **Keyboard** tab to the settings window. It lets the user:
  - choose the modifier used by the two numbered families;
  - record a new chord for each of the other five shortcuts;
  - reset any of them to its default.
- Persist the customizations as preferences. A change takes effect
  immediately, including the key equivalent shown beside Window > Command
  Center.
- Reject any customization that would collide with another Knot shortcut, or
  that has no ⌘, ⌃ or ⌥ modifier.

Non-goals:
- Making the other built-in shortcuts customizable, such as the Agents menu's,
  the permission prompt's, or ⌘Q.
- Adding menu items for the new shortcuts.
- Leaving a shortcut unbound.
- Supporting multi-keystroke sequences.

## Capabilities

### New Capabilities

- `keybindings`: the navigation shortcuts, their defaults, what each does, how
  the user customizes them, and the validation rules for a customization.

### Modified Capabilities

- `app-menu`: Window > Command Center's key equivalent becomes the
  user-configurable Command Center shortcut (default unchanged, ⌥⌘0).
- `settings-ui`: the tab strip gains an eighth tab, Keyboard.

## Impact

- `knot-core`: new keybinding preference fields on `Settings`, decode-tolerant
  with defaults; new l10n keys.
- `knot`: a new `keymap` module that owns the configurable bindings (build,
  validate, rebind at runtime) and their handlers. The workspace window gains
  per-window handlers for agent selection, focus-input and the two panel
  toggles. The settings window gets a new Keyboard pane. `app_bootstrap`
  installs the bindings from settings instead of hard-coding ⌥⌘0.
- The Swift reference has ⌘1–⌘9 for workspaces and ⌘0 for the Command Center.
  Numbered agent selection, the panel and focus shortcuts, and customization
  are all new behavior.
