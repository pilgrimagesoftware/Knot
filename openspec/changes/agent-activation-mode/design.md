## Context

See proposal.md — Why. The constraints that shape the approach:

- Starting an agent today is implicit. `WorkspaceWindow` calls
  `ensure_session` / `ensure_panel_session` from several places - window
  open, row click, dashboard card, panel repaint - and each is a no-op if a
  session already exists. There is no single "start this agent" decision to
  put a condition on, so this change has to create one.
- Activation mode is durable; whether an agent has *been* activated is not.
  The two must not be confused: `agent-lifecycle` already separates durable
  from runtime fields and lists exactly what resets on load.
- `SavedAgent` is deserialized from settings files written by earlier builds
  and by the Swift app. Every field added since has used `#[serde(default)]`,
  and the default it lands on is a behavioural decision, not a formality.
- There is no in-app way to stop a session and keep the agent. `remove_agent`
  removes it; `restart` recreates it. Deactivation is a third operation.

## Goals / Non-Goals

**Goals:**

- One place that decides whether an agent may start, which every existing
  caller of `ensure_session` / `ensure_panel_session` routes through.
- A deactivation path that reuses the teardown `remove_agent` already does,
  without the removal.
- Behaviour for existing settings files that is identical to today.

**Non-Goals:**

- Changing what happens once an agent is running. An activated passive agent
  is an ordinary running agent.
- A bulk control ("deactivate all", "start everything"). One agent at a time
  through the existing menu.
- Showing activation mode on the sidebar row or dashboard card. Worth doing,
  but it is a separate visual-design question and this change is already wide.
- Any change to how sessions are resumed. `restore-conversation-on-launch`
  keeps working as specified; a passive agent simply resolves its resume id
  later, when it activates.

## Decisions

### Activation is gated at a single `should_start` seam, not at each call site

`ensure_session` and `ensure_panel_session` are called from at least four
places. Adding `if mode == Active` to each would leave the next caller to
remember, and the bug it produces - an agent that starts when it should not -
is invisible until someone counts subprocesses.

Instead both functions begin with one check: has this agent been activated in
this run? Selection is the only thing that sets that flag, plus the workspace
open path for `active` agents. Every existing caller keeps working unchanged
and gets the new behaviour for free.

*Alternative considered:* a separate `activate(id)` entry point that callers
must use instead of `ensure_*`. Rejected - it is the same "remember to call
the right one" problem, and the repaint poll legitimately wants "ensure it is
running if it should be".

### `activated` is a runtime flag on `Agent`, not a second persisted mode

A passive agent that was running when the app quit must come back passive, or
"passive" would decay into "active after the first use". A runtime-only
`activated: bool`, listed in the spec's runtime-field reset alongside `state`
and `registered`, gets that for free.

*Alternative considered:* deriving activation from "does a session exist for
this id". Rejected - it cannot distinguish *not started yet* from *stopped by
the user*, and the second must not restart on the next repaint.

### Legacy records load as `active`, new agents default to `passive`

These two defaults disagree on purpose, and it is the most consequential
decision here.

`#[serde(default)]` on the new field would give every existing agent
`passive` - the enum's natural default and the one the dialog wants - and the
user's next launch would open a workspace where nothing starts. That reads as
a broken update, not a feature.

So the serde default is `active` (an explicit `default = "..."` function, with
the reason in a comment beside it), while `CreateOptions` defaults the field to
`passive`. Written down in the spec as two separate scenarios, because the
discrepancy looks like a bug to anyone who finds only one of them.

*Alternative considered:* a one-time migration stamping `active` onto existing
records. Rejected - it needs a schema-version marker the settings file does
not have, and the serde default achieves the same thing with nothing to run.

### Deactivation reuses the teardown, not the removal

`remove_agent` already tears a session down correctly: `remove_session`, drop
the panel state, the prompt input, its subscriptions, the scroll handle,
pending context. Deactivation is that sequence without `store.remove` and
without touching workspace membership.

The shared part is extracted into one function that both call, so a future
field added to the teardown cannot be added to one path and missed in the
other - which is exactly how a session leak of this shape would arrive.

Companions deactivate before their owner, mirroring the removal cascade.

### Selecting a deactivated agent starts it, in either mode

Otherwise a deactivated `active` agent has no way back short of Restart or
reopening the workspace, and "Deactivate" becomes a trap. Selection is
already the universal "I want this one" signal.

This works without extra state because activation happens on the *selection
event*, not continuously: deactivating from the context menu does not
re-trigger the click handler, so the agent stays down until the row is
clicked again.

### The dialog's control is a segmented control, not a switch

A switch needs a label that reads correctly in both positions, and
"Active ⟷ Passive" is not an on/off pair - neither is the absence of the
other. Two named segments say what both choices are without the user having
to try one.

The hint below uses the settings window's `hint()` helper, per the project's
UI conventions, and says one sentence per mode.

## Risks / Trade-offs

- **A passive agent looks broken before its first activation.** Its pane is
  empty and its state reads Idle, which is what a hung agent looks like too →
  the pane gets an explicit stopped placeholder naming why it is not running
  and how to start it. This is the same failure the dialog hint guards
  against, from the other side.
- **Deactivating loses in-flight work.** An ACP turn is cut off mid-response,
  and there is no confirmation → accepted deliberately, and written into the
  spec: the agent can be selected again, and a confirmation on a reversible
  action trains people to dismiss confirmations on irreversible ones. Revisit
  if it bites in practice.
- **The two defaults will confuse a future reader.** Someone will find
  `default = active` in `knot-core` and "new agents are passive" in the
  dialog and assume one is a bug → the comment at the serde default and the
  two spec scenarios exist for that reader.
- **Interaction with layout restore.** Restoring a layout selects an agent,
  which would activate a passive one - arguably right, arguably a back door
  around the whole feature → restore sets the selection without going through
  the activation path, so a restored passive agent stays stopped until the
  user actually clicks it. Called out in tasks so it is verified, not assumed.

## Migration Plan

No migration step. Existing settings files deserialize to `active` and behave
exactly as before; the first agent created after the update is the first
`passive` one. Rolling back leaves an unknown `activationMode` key in the
file, which the previous build ignores.

## Open Questions

- Should the sidebar mark a passive-and-not-yet-started agent - a dimmed row,
  a dot - so the user can see which agents are idle by choice? Deferrable: it
  changes no requirement here and no task below, and is better answered by
  looking at a real workspace of mixed agents than by guessing now.
