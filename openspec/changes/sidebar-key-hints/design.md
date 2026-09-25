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

### ⌘-hold hints: window state, a timer, and a label set formatted once

The workspace window holds a `KeyHintHold` (`workspace_window/key_hints.rs`):
whether a hold is armed, a generation counter, and the `SidebarKeyHints`
labels once they are showing.

- The root element's `on_modifiers_changed` arms the hold when ⌘ goes down and
  ends it when ⌘ comes up. Other modifiers do neither, so adding ⌥ keeps the
  hints.
- A `capture_key_down` listener on the root ends the hold, so ⌘C cancels it.
  It does not stop propagation. It is in the capture phase because a focused
  input handles most keys and stops them bubbling.
- Arming spawns a 500 ms timer (`consts::KEY_HINT_DELAY`). When it fires it
  checks that the hold it was started for is still the current one, by
  generation, before showing anything. A per-frame elapsed-time check would
  need the window to keep repainting while nothing else changes.
- The frame's prepare pass ends the hold when the window is not active. gpui
  refreshes a window when its activation changes, so this runs on
  deactivation. macOS does not deliver the ⌘ key-up to a window that lost key
  status during ⌘Tab.

Modifier changes and key-downs are dispatched along the focused element's
path, innermost first. The composer's input listens for modifier changes but
does not stop propagation, and the terminal registers no such listener, so the
root receives both with either pane focused. This was checked in task 2.1, and
`key_hints_tests` covers the composer case.

The labels are formatted from `Resolved` when the timer turns the hints on,
and kept until the hold ends. The render path formats nothing and reads no
settings, per the no-work-on-the-render-path rule in
`.claude/rules/rust-structure.md`. A rebinding cannot land mid-hold: recording
one needs the settings window focused, which ends the hold. That is simpler
than rebuilding on every keymap apply, which the first version of this design
proposed, and it is equivalent.

Rendering: `with_key_hint` adds an absolutely positioned badge to an element,
which is what keeps row sizes unchanged.
- At full width the badge sits against the row's trailing edge, vertically
  centred.
- In the compact layout it goes over the corner of the avatar or icon. The
  compact agent row puts it on a box around the avatar tile, because the tile
  clips to its bounds.
- The New agent control uses the trailing-edge placement at both widths. From
  a row as wide as the sidebar, a corner badge would overhang the sidebar's
  edge.

Alternative: show hints for as long as ⌘ is held, with no delay. Rejected. The
hints would flash on every ⌘C, ⌘V and ⌘Tab.

Alternative: show hints only while exactly the family's modifier is held (⌥⌘
for agents). Rejected. Users would have to know the chord before the hint could
show it to them.

## Risks / Trade-offs

- [gpui may not deliver `ModifiersChangedEvent` to a window whose focused
  element is the terminal pane] → The listener is on the root element, which
  is an ancestor of every pane. Task 2.1 verifies this with the terminal
  focused before any rendering work starts.
- [⌘T in the terminal pane: a shell user may expect it to reach the program]
  → The terminal forwards no ⌘ chords today (they are app shortcuts on macOS),
  so ⌘T takes nothing that currently works.
