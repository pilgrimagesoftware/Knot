# Proposal

## Why

Sending is a binary gate: the Send button is disabled for the whole time a
response streams, so a user who thinks of the next thing while the agent is
answering must wait for silence, then type it up into a composer that had
nothing better to do. For a panel used to steer a working agent, the quiet
gap is small but the friction is out of step with how the agent itself works
- a real agent queues and keeps moving. Prompt queueing lets the user keep
queueing turns in order while the current one plays out.

## What Changes

- The Send control stays enabled while a response is in progress. A prompt
  sent then is **enqueued** rather than refused.
- Queued prompts are delivered **in order**, each starting its own turn as
  the previous one ends, until the queue empties. When nothing is in flight
  the send path is unchanged: the prompt is delivered immediately.
- A queued prompt is visible in the conversation, clearly marked as waiting,
  and reads as a normal prompt once delivered.
- A prompt suppressed by a pending **permission request still cannot be sent**
  - that gate is about the agent waiting on an answer, not about turn
  timing, and is unchanged.
- A queued prompt whose delivery fails is reported under that prompt (the
  existing error path) and the rest of the queue continues.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `acp-panel-ui`: the input-area send control changes from "disabled during a
  response" to "enqueues while a response is in progress", and a prompt-queue
  requirement is added covering ordering, queued presentation, and delivery
  handling.

## Impact

- `crates/knot`: `panel_state` gains the queue (pure state: enqueue, dequeue
  on turn end, error handling), `panel_session`'s drain task performs the
  flush (it owns both the session and the state), and `panel_view` renders
  queued prompts distinctly. The workspace window's `send_panel_prompt` gates
  loosen from `!turn_active` to `no pending permission`.
- No wire or protocol change; queued delivery still uses `session/prompt`.
- User-facing strings go through `knot_core::l10n::t`.