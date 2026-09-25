# Design

## Context

- `view-menu-navigation` moved the workspace-scoped shortcuts onto the
  workspace window's root element and keeps focus inside it, so a handler
  registered there is on the dispatch path whenever the window is focused.
- The sidebar is drawn in `workspace_window/render/` (`mod.rs` for the title
  bar, New agent control and column; `sidebar.rs` for the rows). Labels for
  chords come from `keymap::Chord::label`, the glyph form macOS menus use.
- The effective bindings are `keymap::Resolved`, applied by `keymap::apply`.

## Goals / Non-Goals

**Goals:**
- Hints that never change a row's size and cost nothing on the render path.

**Non-Goals:**
- Hints anywhere but the sidebar.
- Configurable New Agent key.

## Decisions

### New Agent is a fixed ⌘T shortcut on a global handler

`NewAgent` joins `keymap/fixed.rs`, so the validator already rejects a
customization onto ⌘T. Its handler is registered on the workspace window's root element, as the
navigation shortcuts' are since `view-menu-navigation`, so File > New Agent…
is disabled wherever no workspace window is focused, through action
availability. `MenuItem::disabled` has no effect on an item whose action has a
handler (see that change's design).

Alternative: make it configurable. Rejected. Nothing asked for it, and the
reference's key is fixed.

### ⌘-hold hints: window state, a timer, and a cached label set

The workspace window tracks `command_held_since: Option<Instant>` and
`show_key_hints: bool`:

- The root element's `on_modifiers_changed` sets `command_held_since` when ⌘
  goes down and clears both when it goes up.
- A key-down listener in the capture phase clears both, so that ⌘C cancels the
  hold. It does not stop propagation.
- `show_key_hints` turns on from a 500 ms timer spawned when ⌘ goes down. The
  timer checks that the hold it was started for is still the current one and
  then notifies the view. A per-frame elapsed-time check would need the window
  to keep repainting while nothing else changes.
- Window deactivation (`observe_window_activation`) clears both. macOS does not
  deliver the ⌘ key-up to a window that lost key status during ⌘Tab, so without
  this the hints would still be showing when the user returns.

The labels come from `Resolved`, stored on the window as a `SidebarKeyHints`
(Dashboard, Pull Requests, New Agent and nine agent chords as strings). It is
rebuilt when the keymap is applied, the same moment the menu bar is rebuilt, so
the render path formats nothing and reads no settings. This follows the
no-work-on-the-render-path rule in `.claude/rules/rust-structure.md`.

Rendering: at full width a hint is an absolutely positioned, right-aligned
label inside the row, over its trailing content. In compact layout it is an
absolutely positioned badge at the avatar's or icon's corner. Being absolutely
positioned is what keeps row sizes unchanged.

Alternative: show hints for as long as ⌘ is held, with no delay. Rejected. The
hints would flash on every ⌘C, ⌘V and ⌘Tab.

Alternative: show hints only while exactly the family's modifier is held (⌥⌘
for agents). Rejected. Users would have to know the chord before the hint could
show it to them.

## Risks / Trade-offs

- [gpui may not deliver `ModifiersChangedEvent` to a window whose focused
  element is the terminal pane] → The listener is on the root element, which
  is an ancestor of every pane. Task 5.1 verifies this with the terminal
  focused before any rendering work starts.
- [⌘T in the terminal pane: a shell user may expect it to reach the program]
  → The terminal forwards no ⌘ chords today (they are app shortcuts on macOS),
  so ⌘T takes nothing that currently works.
