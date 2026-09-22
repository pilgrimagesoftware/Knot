# Design

## Context

`knot_messaging::routing::send` and the `send_message` MCP tool
(`crates/knot-mcp-tools/src/messaging.rs`) run off the GPUI main thread's
window state entirely: they take `&AgentStore` (immutable) and a
`&dyn DeliveryNotifier`, and know nothing about any `WorkspaceWindow`.
Starting an agent's session, by contrast, is entirely window-owned -
`WorkspaceWindow::ensure_session` (`crates/knot/src/workspace_window/sessions.rs`)
spins up the panel/terminal session and lives on `self.panel_sessions`, a
field of the window. `AgentStore` itself has `deactivate` (flips
`agent.activated = false` and tears down nothing itself - the window does
that) but no matching `activate`; there is no store-level notion of
"start a session" to call from routing.

The codebase already has one cross-thread request of exactly this shape:
`AwaitingInput` (`crates/knot/src/app_support.rs`) is a
`gpui_kit::Global`-registered `Arc<Mutex<Vec<(Uuid, Option<String>)>>>` that
the MCP tool catalog pushes into from a hook callback, and that each
`WorkspaceWindow` drains on its repaint poll to raise a desktop notification
for whichever agent it owns. `panel_needs_repaint`
(`crates/knot/src/workspace_window/repaint.rs`) already runs a poll loop over
every agent with something queued (`drain_panel_prompt`) each frame. Both
are precedents for "a background/MCP-thread event becomes a per-window
action the next time that window polls."

## Goals / Non-Goals

**Goals:**
- Let `send_message`'s delivery path request that a deactivated recipient be
  activated, without giving the messaging/store layer a dependency on GPUI
  or on `WorkspaceWindow`.
- Have the activation actually happen - the session starts - not just flip a
  flag that nothing acts on.
- Keep `knot-messaging` (routing, delivery, the nudge) decoupled from how
  activation is carried out, matching its existing shape: it already takes a
  `&dyn DeliveryNotifier` trait object rather than reaching into the window
  layer directly for the nudge.

**Non-Goals:**
- Activating from `broadcast` - proposal.md scopes that out explicitly.
- Changing what activation *does* once triggered (`ensure_session`'s
  behavior, activation-mode semantics) - this only adds a new trigger for
  the existing start path.
- A synchronous guarantee that the recipient's session is running by the
  time `send_message` returns. Session start is asynchronous today (spawned
  work, ACP handshake); this change does not make it synchronous, matching
  how selecting a deactivated agent in the UI doesn't block on its session
  coming up either.

## Decisions

**An `ActivationQueue` global, mirroring `AwaitingInput`.**

Add `pub(crate) type ActivationQueue = Arc<Mutex<Vec<Uuid>>>;` and an
`Activation(pub(crate) ActivationQueue)` `gpui_kit::Global` next to
`AwaitingInput` in `app_support.rs`. `send_message` pushes the recipient's id
onto it exactly when routing reports the delivery succeeded and the
recipient's `activated` flag was false. `panel_needs_repaint` drains it
alongside the existing `waiting` / `drain_panel_prompt` loop: for each queued
id whose agent belongs to this window's workspace, call the same
`ensure_session` the sidebar's click handler calls, then `cx.notify()` (the
window's existing `AgentRow` snapshot already reads `agent.activated` off
the store each render, so no separate signal is needed for the row to show
the agent as running once the store reflects it).

*Alternative considered*: give `AgentStore` itself a synchronous `activate`
that starts the session inline. Rejected - session start needs the tokio
runtime handle and the panel-session map that only `WorkspaceWindow` owns
(`self.runtime`, `self.panel_sessions`); moving those into `AgentStore` would
turn a pure data store into something that owns live process/session state,
which is the same layering `deactivate` already avoids (it flips the flag
and lets the window tear the session down).

*Alternative considered*: route the activation request through
`DeliveryNotifier` (the trait `knot-messaging` already calls for the nudge),
adding an `activate(&self, agent_id: Uuid)` method. Rejected for this change
- `DeliveryNotifier` is implemented once, in the app crate, specifically to
send the ACP inbox prompt; overloading it with a second, unrelated kind of
side effect (start a session vs. nudge one that's running) makes one trait
answer two different questions. A queue the tool layer pushes to and the
window drains matches the *existing* AwaitingInput/panel-prompt-results
shape instead of introducing a new abstraction.

**Activation triggers on the store flag, not on session absence.**

`send_message` checks `recipient.activated == false`, not whether a live
session exists for that id. `activated` is exactly the field `deactivate`
clears and the field `agent-lifecycle`'s "deactivated agent SHALL NOT start
again on its own" requirement is stated in terms of - checking session
presence instead would also fire for an agent whose session simply hasn't
been created yet for unrelated reasons (e.g. a passive agent never
selected), which is a state this change is not meant to touch: a passive,
never-started agent getting a direct message should behave exactly as it
does today (queue, deliver the nudge once idle) unless and until this change
gives it a reason to start - which it now does, since an un-started passive
agent also has `activated == false`. Scoping on the flag rather than a
separate "was this specifically deactivated" bit is deliberate: the
lifecycle spec already treats "never started" and "deactivated" as the same
state for the purposes of "does this agent need to be started", and this
change's activation trigger follows that.

**No new field on `Message` or `QueuedPanelPrompt`.**

Activation is a side effect of a successful direct send, resolved once at
delivery time - it doesn't need to be remembered alongside the message
itself the way `PromptOrigin` does (which exists because the message's
*wording* alone can't tell a nudge from a user prompt after the fact).
Nothing downstream needs to know later whether a given message triggered
activation.

**Activation is the flag plus both `ensure_*`, drained beside the repaint
poll rather than inside it.**

Two corrections to the sketch above, found while implementing:

`ensure_session` alone cannot start a message recipient. It is the PTY path
and returns early for anything `runs_a_terminal_process` rejects, and
messaging already refuses shell agents as recipients - so every recipient
this change can reach starts through `ensure_panel_session` instead. Both
also return early while `agent.activated` is clear, which the store flag
only leaves set through `set_activated`. So the drain does exactly what the
sidebar's click handler does minus the selection: `set_activated(id, true)`,
then `ensure_session` and `ensure_panel_session`. Neither half alone starts
anything, which is why the spec's "the same way selecting it in the UI does"
is the whole instruction. The selection itself is deliberately not changed -
a message addressed to a background agent is not a request to change what
the user is looking at.

The drain lives in `WorkspaceWindow::activate_messaged_agents`
(`workspace_window/agents.rs`), called from the repaint poll in `open.rs`
immediately before `panel_needs_repaint`, with its return folded into the
same `cx.notify()` condition. Not inside `panel_needs_repaint`, which takes
no `Context` and so cannot reach a global; this matches
`raise_awaiting_notifications`, the existing queue drain, which is a sibling
of the poll for the same reason. Ordering it before the repaint checks means
a session started this frame has its slot in place when they run.

The claim step is a free `claim_activations` function so the put-back is
testable without a window: dropping an entry this window does not own would
strand a stopped agent whose own window merely had not polled yet, and it
would fail silently - the message is already delivered, and nothing else
ever starts the recipient.

## Risks / Trade-offs

- **[Risk]** The activation queue could grow if a workspace's window never
  polls (window closed, minimized without a repaint loop) while messages
  keep targeting a deactivated agent. → Mitigation: push is idempotent
  per-id-until-drained is unnecessary to add explicitly, since duplicate
  entries for the same id are harmless (`ensure_session` on an
  already-starting/started agent is a no-op) and the queue is bounded in
  practice by how many distinct deactivated agents exist in a workspace, the
  same bound `AwaitingInput` already accepts.
- **[Risk]** A recipient could be re-deactivated by the user between the
  send succeeding and the window draining the activation queue, racing the
  two. → Mitigation: `ensure_session` already has to handle "agent no longer
  wants to be running" defensively for the existing UI path (a user can
  click deactivate immediately after selecting), so this introduces no new
  race the window doesn't already resolve.
- **[Trade-off]** The recipient's session start is asynchronous and
  `send_message`'s response doesn't wait for it, so a sender that messages a
  deactivated agent and immediately checks for a reply will find none yet -
  already true of a normal busy recipient, and `send_message`'s response
  text already tells the caller not to expect an immediate reply.
