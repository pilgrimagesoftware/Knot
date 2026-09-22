# Proposal

## Why

Selecting an agent puts its conversation on screen with the cursor nowhere. The
user clicks the agent, types, and nothing happens — they have to click the
composer first. Every message to every agent costs an extra click, and the cost
is highest exactly where the app is meant to be fastest: switching between
several agents in one workspace.

The app already knows how to do this. The composer is focused when a queued
prompt is pulled back into it (`take_queued_prompt_into`), and the window
already sets focus during frame preparation so the Agents menu has a dispatch
path. What is missing is the connection between "this agent is now selected" and
"its composer is where typing goes".

## What Changes

- Selecting a Panel-mode agent gives its prompt input keyboard focus, whether
  the selection came from clicking the sidebar row, clicking the agent's card in
  the window's overview, creating the agent, or restoring the selection when the
  workspace window opens.
- Focus is taken once per selection, not held. Once the user moves focus
  elsewhere in the window — the terminal pane, a config selector, a control on a
  queued message — it stays where they put it until the selection changes again.
- Focus is not taken when the composer is not on screen: the window showing the
  dashboard, a markdown or diagram pane holding the content area, a deactivated
  agent showing the stopped placeholder, or a terminal-mode agent.

Non-goals:

- **Focusing the terminal pane when a terminal-mode agent is selected.** It is
  the same complaint and deserves the same fix, but focusing a terminal routes
  every subsequent keystroke to the PTY, which changes what ⌘C does and which
  menu items are enabled (`app-menu`'s Edit-menu requirement turns on exactly
  when a text field has focus). That is a behavioral decision about key routing,
  not a focus convenience, and it belongs in its own change.
- Changing what selection does otherwise. It still activates a passive agent and
  still starts the session, per `agent-lifecycle`.
- Any change to the composer itself — its placeholder, its send chord, its
  expand and collapse, its queue.
- Raising or activating the window. This sets focus inside a window; it never
  brings one forward.

## Capabilities

### Modified Capabilities

- `acp-panel-ui`: gains a requirement for where keyboard focus lands when a
  Panel-mode agent is selected, including the cases where it deliberately does
  not move. The capability already specifies the input area's send control,
  expansion, queue and context attachment; where focus goes on selection is the
  one thing about it left unsaid, which is why it does nothing today.

### New Capabilities

None.

## Impact

- `crates/knot/src/workspace_window/render/mod.rs` — `prepare_frame` already
  holds the window's frame-time focus logic and is where this lands. It needs to
  tell "the selection changed" from "nothing is focused", which the existing
  `window.focused(cx).is_none()` guard cannot do.
- `crates/knot/src/workspace_window/window.rs` — one field recording which
  agent's composer was last focused, so focus is taken once rather than every
  frame.
- `crates/knot/src/workspace_window/panel/prompt.rs` — `panel_prompt_input`
  already gets-or-creates the per-agent `TextareaState`; nothing new is needed
  to reach it.
- `crates/knot/src/workspace_window/agents.rs` — `select_agent` takes neither
  `window` nor `cx` today, and its four callers are click and menu handlers.
  Deciding focus at frame preparation rather than in `select_agent` keeps that
  signature and keeps all four paths consistent for free.
- No new dependency, no persistence change, no user-facing text.
