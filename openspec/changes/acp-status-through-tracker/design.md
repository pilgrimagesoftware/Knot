# Design

## Context

`sync_panel_agent_states` runs once per repaint poll and derives each ready panel
session's status from two flags it reads under the session's lock:
`pending_permission.is_some()` wins, then `turn_active`, else Idle
(`crates/knot/src/workspace_window/repaint.rs:213`). It is a *level* reading —
every tick recomputes the same answer — while `Tracker` takes *edges*: each
`apply_acp_status` call is a reported transition, and `ActivityState::apply_acp_status`
pushes `Effect::AwaitingInput` every time it is handed `Input`
(`crates/knot-activity/src/state/model.rs:256`), not only on a change.

That mismatch is the design's central constraint. Everything else follows from
where the tracker can be built and what it needs to reach.

`Tracker::spawn` needs a tokio runtime. `WorkspaceWindow` holds one and already
takes `self.runtime.enter()` guards where it starts tokio work
(`workspace_window/sessions.rs:87`). The sink needs the agent store and the
`AwaitingInputQueue` global, both of which the window has and `panel_session.rs`
does not.

`PermissionRequest` carries no free-text prompt. Its human-readable field is
`tool_call_title: Option<String>` (`crates/knot-acp/src/protocol/mod.rs:309`).

## Goals / Non-Goals

**Goals:**

- ACP transitions reach `Tracker::apply_acp_status`, so `Effect::AwaitingInput`
  and `Effect::CheckMessages` fire for Panel-mode agents.
- Exactly one notification per permission request, not one per poll tick.
- No second status route: the tracker's `on_status` sink becomes the only writer
  of a Panel agent's `AgentState`.
- No new file over 700 lines, and no edit to `workspace_window/panel/`, which
  issue #323 and issue #411 are both working in.

**Non-Goals:**

- Changing `knot-activity`. `apply_acp_status` and its effects are written and
  tested; this change supplies the caller.
- Distinguishing "turn ended" from "agent asked a question". ACP cannot.
- Touching the hook or terminal status routes.

## Decisions

### The poll converts level to edge; the tracker keeps taking edges

`sync_panel_agent_states` already computes whether an agent's state differs from
the store's (`moved |= store.agent(id).is_some_and(|agent| agent.state != state)`).
That comparison becomes the gate: the tracker is called only for agents whose
derived state differs from what the store currently holds. A permission request
that stays pending for thirty ticks reports once.

The alternative — dedupe inside `ActivityState` by suppressing `Effect::AwaitingInput`
when the status is already `Input` — was rejected. It would change behavior for
the hook route, where a second event for the same prompt is already meaningful
enough that `desktop-notifications` names it explicitly, and it would hide the
level/edge mismatch in the crate that is not causing it. `notifications.rs`'s
`notified_awaiting` map is a third guard and stays as a backstop; it is keyed by
agent and message, so it cannot catch a repeat that a second window also sees.

One consequence worth stating: the store read that used to be a dirty-check is
now load-bearing. If the store write ever stopped happening, the gate would fire
every tick. That is why the sink writing the store and the gate reading it must
stay the same value — see the next decision.

### The tracker's sink is the only writer of a Panel agent's state

`sync_panel_agent_states` stops calling `store.set_state`. It calls
`tracker.apply_acp_status(state, message)`, and the tracker's `on_status` sink
writes the store, exactly as `knot-mcp-tools` already does for hook-driven agents
(`crates/knot-mcp-tools/src/lib.rs:155`).

Two writers was the alternative: keep the direct write for the dot's latency and
add the tracker call for its effects. Rejected — the tracker's channel is
asynchronous, so the two would race, and a status the tracker later re-derives
(`mark_idle`, a process exit) would fight the poll's next tick. One writer means
the dot is one tracker hop behind where it is today; the hop is a bounded channel
send consumed by a task on the same runtime, and the next poll tick redraws.

### The tracker map lives on `WorkspaceWindow`, keyed by agent

`panel_trackers: BTreeMap<Uuid, knot_activity::Tracker>`, declared in
`workspace_window/window.rs` beside `panel_prompt_queues`, initialised in
`workspace_window/open.rs`, and removed in the per-agent teardown in
`workspace_window/sessions.rs:298` where every other per-agent panel map is
already dropped.

Built lazily, on the first tick that sees a session in `PanelSessionSlot::Ready`,
under a `self.runtime.enter()` guard — the same shape `knot-mcp-tools::tracker_for`
uses, including its `Handle::try_current()` bail. Lazily rather than at session
creation so this change does not edit `workspace_window/panel/session.rs`, which
#411 has uncommitted work in.

Hanging the tracker off `PanelSessionHandle` was the alternative. It puts the
tracker next to the events that drive it, but `panel_session.rs` has neither the
store nor the notification queue, so the sink would have to be injected from the
window anyway — and it would collide with #323 and #411.

### Tracking preset is `ACP_UPDATES`, explicitly

`tracking_for(agent_type, ViewMode::Panel)` returns `ACP_UPDATES`
(`crates/knot-activity/src/tracking.rs:57`). `knot-mcp-tools` hardcodes
`ViewMode::Terminal` at its own call (`lib.rs:172`), correctly for the hook path
it serves; this one passes `Panel` and must not be refactored to share that call.
`ACP_UPDATES` is what keeps the idle timer and the input-protection guard off
these transitions, which `activity-detection` requires.

### The attention message is `tool_call_title`

`PermissionRequest::tool_call_title` is the only human-readable field on the
request, so it is the notification body. `None` carries no message and the
notification falls back to "Needs your attention", which the spec already
requires. `options` is a list of button labels and says nothing about what is
being asked.

## Risks / Trade-offs

**The dot is one channel hop slower.** Accepted, as above. If it ever shows,
the fix is to make the poll's tick cadence the bound, not to restore the second
writer.

**A tracker that fails to spawn leaves an agent with no status.** `tracker_for`
returns false when no runtime handle is current, and `knot-mcp-tools` falls back
to a direct `set_state`. The same fallback applies here: no tracker, direct
write, no effects — degraded rather than frozen. Worth a test, because the
fallback is the branch that reintroduces the bug if the guard is ever wrong.

**Agents in two windows.** The `AwaitingInputQueue` is a global partitioned by
`workspace_agent_ids` in `raise_awaiting_notifications`, so a queue entry is
claimed by the window that owns the agent. A per-window tracker map is therefore
safe: an agent belongs to one workspace window, and its tracker to that window.

**`sync_panel_agent_states` grows.** `repaint.rs` is well under the cap, but the
lazy construction and the gate are enough to want the tracker lookup in its own
function rather than inline in the loop.
