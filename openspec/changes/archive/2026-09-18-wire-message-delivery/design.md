## Context

See proposal.md — Why. What shapes the approach:

- The workspace window already runs a 33ms poll that drains queues filled off
  the main thread (clipboard writes, exited sessions) and repaints. It is the
  natural home for a check that has to notice an agent becoming idle.
- `should_inject_inbox_prompt(agent_type, mcp_enabled, latest_message,
  last_injected)` is already written and unit-tested, and its shape says how
  this was meant to work: compare the latest unread message id against the
  last one nudged about, rather than react to a delivery event.
- `QueuedNotifier` fills on send when the recipient is idle *at that moment*.
  That cannot carry the requirement on its own - the spec also asks for
  delivery on the recipient's next transition to idle, which no send-time
  event knows about.
- Sending a prompt into a panel session already has a correct gate, used by
  the composer: ready slot, no pending permission, no turn in flight.

## Goals / Non-Goals

**Goals:**

- Deliver the nudge the spec has always described, over the channel agents
  actually have.
- Use the pieces already written rather than adding parallel ones.

**Non-Goals:**

- Changing routing, scoping, or who may message whom.
- A message inbox UI. The nudge tells an agent to call `check-messages`;
  reading a knot's traffic as a human is a different feature.
- Delivering to shell agents. `mcp-messaging` already says they cannot
  receive messages, and `should_inject_inbox_prompt` already excludes them.

## Decisions

### Poll the store, do not consume the notifier queue

`should_inject_inbox_prompt` compares "latest unread" against "last nudged",
which is a state comparison, not an event. That is what the requirement needs:
a message that arrived while its recipient was working has no event left by
the time the recipient goes idle, but the unread message is still there to be
found.

So the poll asks the store what is unread, and `QueuedNotifier` keeps its
existing job of feeding the user-facing delivery notice. Draining it for
delivery as well would give two sources of truth that disagree the moment an
agent is busy.

*Alternative considered:* drain the notifier and re-queue events for busy
recipients. Rejected - it rebuilds, in a queue, the state the store already
holds correctly.

### The guard is the turn, not a keystroke window

The requirement's "input-protection guard" is a terminal concept: do not type
into a terminal the user is typing into. An ACP agent has no such surface. Its
equivalent is the composer's own gate - a ready session, no pending permission,
no turn in flight - which protects the same thing: a prompt arriving in the
middle of something.

Reusing that gate also means the nudge cannot be the thing that discovers a
session is not ready.

### Last-nudged is per agent and runtime-only

It lives beside the other per-agent UI bookkeeping in the workspace window,
not on `Agent` and not in settings. A relaunch that re-nudges about a still
unread message is correct behaviour, not a bug to persist away from.

### The unread badge is left alone

It looked like a free ride - `layout_model` takes unread counts, `AgentRow`
carries one - until the callers turned out not to exist: the sidebar builds
its rows straight from the store and never calls `layout_model`. Showing
unread counts is therefore a row-rendering change, not a populated map, and
belongs with whoever designs how the badge looks rather than riding along
here.

## Risks / Trade-offs

- **The nudge lands as a user-visible prompt in the agent's conversation.** It
  will look like the user typed "check your inbox" → accepted; it is what the
  requirement asks for, and an agent's conversation showing why it went and
  read its messages is more legible than a silent side channel.
- **A busy agent is nudged later than the message arrived.** Up to one poll
  interval after it goes idle → 33ms, immaterial.
- **An agent that never calls `check-messages` is nudged once and then
  never again for that message** → deliberate, per the once-per-message rule.
  The alternative is nagging on every idle tick, which is worse.

## Open Questions

None.
