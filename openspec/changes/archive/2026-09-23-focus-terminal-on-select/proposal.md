# Proposal

## Why

Selecting a shell agent leaves the keyboard focus where it was, so the user
has to click inside the terminal pane before they can type into it. Selecting
a Panel-mode agent does not work that way - `acp-panel-ui` requires its prompt
input to take focus, precisely so an agent can be selected and typed to with
no intervening click. The terminal half of the same gesture was never
specified, and `terminal_focus` is focused from exactly one place: the pane's
own mouse-down handler.

## What Changes

- Selecting a Terminal-mode agent whose grid is on screen gives the terminal
  pane keyboard focus, so the user can type immediately.
- Focus is taken on the frame the grid first appears on, not on the frame the
  selection changes. A session takes a moment to spawn, and until it does the
  pane shows the "Starting terminal…" placeholder with no focusable element -
  focusing then would land on a handle that is not in the element tree, and
  `prepare_frame`'s own "something must hold focus" fallback would move it
  straight to the window root, with no later frame retrying.
- The rules the composer already follows carry over unchanged: taken once per
  selection and never pulled back while the user works elsewhere in the
  window, not taken over an open dialog, not taken when a markdown or diagram
  pane holds the content area or the agent is deactivated, and never raising
  or reordering a window.
- `showing_composer` generalises from "which composer will be shown" to
  "which of the pane's input targets will be shown", since the latch that
  makes "once per selection" work has to cover both or a switch between two
  agents of different modes will miss a transition.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `terminal-input`: adds a requirement that selecting a Terminal-mode agent
  focuses its terminal surface, the counterpart of `acp-panel-ui`'s rule for
  the prompt input.
- `acp-panel-ui`: its "Selecting a Panel-mode agent focuses its prompt input"
  requirement lists a Terminal-mode agent among the cases where focus is not
  taken. That exclusion was describing the absence this change fills, so it
  is replaced by a pointer to the terminal's own rule. Its "A Terminal-mode
  agent does not move focus" scenario keeps its title - archive refuses to
  drop a scenario a MODIFIED block omits - and reads, within a requirement
  about the prompt input, as focus not moving *into a composer*, which stays
  true.

## Impact

- `crates/knot/src/workspace_window/composer_focus.rs` - the pure decision
  function and its facts struct.
- `crates/knot/src/workspace_window/render/mod.rs` - `focus_showing_composer`,
  which acts on it.
- `crates/knot/src/workspace_window/window.rs`, `open.rs`,
  `sessions.rs` - the `focused_composer` latch, which becomes a latch over
  either target.
- No new dependencies, no persisted state, no change to how the terminal
  handles keys once it has focus.
