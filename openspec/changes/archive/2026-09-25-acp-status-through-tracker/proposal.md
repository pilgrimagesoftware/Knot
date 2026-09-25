# Proposal

## Why

A Panel-mode agent that stops to ask the user for permission raises nothing. The
status dot turns blue and that is the whole signal — no desktop notification, no
matter what `desktop_notifications_enabled` says. Since `agent::view_mode_for`
makes every non-shell agent Panel-mode, and shell agents report no activity at
all, that means no agent this build can create ever raises an awaiting-input
notification.

`openspec/specs/desktop-notifications/spec.md` requires one: "When an agent
enters Awaiting input and `desktop_notifications_enabled` is on, the system SHALL
raise a desktop notification". The requirement is not scoped to hooks, and the
state machine that would satisfy it is already written and tested.

The break is a missing call, not missing behavior. `ActivityState::apply_acp_status`
sets the status, pushes `Effect::AwaitingInput(message)`, and routes an ACP idle
through `mark_idle` so `Effect::CheckMessages` fires
(`crates/knot-activity/src/state/model.rs:246`). Nothing in the workspace calls
`Tracker::apply_acp_status`. The panel's status reaches the UI by a second route
instead: `sync_panel_agent_states` writes `store.set_state` directly from the
repaint poll (`crates/knot/src/workspace_window/repaint.rs:213`), which skips the
tracker and every effect it would emit.

`apply_acp_status` is `pub` in a library crate, so dead-code analysis cannot flag
it, it carries no `UNWIRED` marker, and no test fails. It is the fourth instance
of the pattern `.claude/rules/rust-structure.md` names: an off-thread result that
never reaches a frame.

## What Changes

- Panel-mode agents get an activity tracker, created with
  `tracking_for(agent_type, ViewMode::Panel)` — the `ACP_UPDATES` preset.
- `sync_panel_agent_states` keeps deriving the state from `pending_permission` /
  `turn_active`, but hands it to `Tracker::apply_acp_status` instead of writing
  the agent store. The tracker's existing `on_status` sink does the store write,
  as it already does for hook-driven agents.
- A permission request's own text travels with the transition as the attention
  message, so the notification body names what is being asked rather than falling
  back to "Needs your attention".
- Entering Awaiting input on an ACP transition therefore raises a desktop
  notification, under the repeat- and visible-agent suppression
  `workspace_window/notifications.rs` already applies.
- A turn ending with no pending permission goes through `mark_idle`, so the idle
  inbox nudge (`Effect::CheckMessages`) fires for Panel-mode agents.

No new state machine, no new status source, and no change to what the dot shows
or when. One pipeline replaces two that could disagree.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `activity-detection`: the ACP requirement gains the tracker as the route those
  transitions take, and states that entering Awaiting input from ACP raises the
  same notification a hook-entered one does. Today the requirement describes the
  status changes alone, which a direct store write satisfies while emitting
  nothing.
- `desktop-notifications`: the awaiting-input notification's body is currently
  specified as "the hook-supplied message". It needs to name the ACP permission
  request's text as the other source, since hooks are not the only way in.

## Impact

- `crates/knot/src/workspace_window/repaint.rs` — `sync_panel_agent_states` calls
  the tracker rather than the store.
- `crates/knot/src/panel_session.rs` and its slot type — a tracker per Panel
  session, built where the session is, and dropped with it.
- `crates/knot/src/app_bootstrap.rs` — the `AwaitingInputQueue` already reaches
  `knot-mcp-tools`; the Panel tracker's sink needs the same queue.
- `crates/knot-activity` — no production change expected. `apply_acp_status` and
  its effects exist and are tested; this change gives them a caller.

**Non-goals.** No new signal for "the agent ended its turn with a question": ACP
reports `end_turn` either way and cannot distinguish a finished agent from a
waiting one. The Swift reference got that cue from Claude Code's `Notification`
hook while running in a terminal, which this port no longer does; recovering it
is a separate decision.

Not reviving the Terminal-mode hook path, which is unreachable while
`view_mode_for` keeps every non-shell agent in Panel mode. Not changing the
Awaiting-input colour — the port's blue divergence from the Swift reference's red
is deliberate (`crates/knot/src/app_state.rs:56`). Not touching the terminal or
hook status routes, which keep their current behavior.
