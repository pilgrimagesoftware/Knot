# Design

## Context

See proposal.md - Why. The mechanics this rests on:

- `send_panel_prompt` gates on `state.pending_permission.is_none() && !state.turn_active`; when the gate fails it restores the pending context and returns - the typed text never leaves the composer.
- `PanelState` is pure: `push_user_message` records a prompt and sets `turn_active = true`; `SessionUpdate::TurnEnd` sets `turn_active = false`; `pending_permission` blocks senders today.
- `PanelSessionHandle`'s drain task owns both the `AcpSession` and the `Arc<Mutex<PanelState>>`, applying each event in the session's ordered stream - the only place that can both see a turn end and trigger the next `session.prompt`.
- `PanelMessage::User` is a plain text variant today; nothing marks a prompt as not yet delivered.

## Goals / Non-Goals

**Goals:**

- The composer never silently swallows a typed prompt again; a busy agent
  means "waiting", not "refused".
- Ordering is the queue's whole contract: FIFO, one turn at a time, nothing
  overtakes anything.
- All queue logic is pure state; only the send call is IO, and it stays where
  sending already lives.

**Non-Goals:**

- No parallel delivery or concurrency against the agent; "prompt" is still a
  turn, one at a time.
- No cancel, reorder, or per-prompt priority - the queue is a FIFO only.
- No queue persistence: a closed session drops its queue.
- No change to permission semantics; the permission gate holds the queue as
  it holds everything else.

## Decisions

### The queue lives in `PanelState`, as pure state
`queued_prompts: VecDeque<String>` alongside the existing turn machinery,
with `enqueue_prompt`, `dequeue_for_delivery`, and `queued_count` helpers.
Pure state means the queue is unit-testable without a session, and the apply
path stays the single source of truth for what the conversation holds. The
task-safety review that applies to the rest of `PanelState` applies unchanged.

### Turn end promotes the next queued prompt and flags delivery
In `apply`'s `TurnEnd` arm, the turn closes as today; if the queue is
non-empty, the head is popped, `push_user_message`-style recorded (which sets
`turn_active = true` and re-enables tracking), and a one-shot
`pending_delivery` is set. `apply` stays pure - it records *that* a delivery
is owed, not the awaiting itself. The drain task reads `take_pending_delivery`
after each apply and calls `session.prompt` for it, so a turn-end that is
immediately followed by further streamed events cannot double-deliver.

### The drain task performs the flush
The queue must flush exactly when the owning session has ended a turn, and the
drain task is the only place that both applies events and holds a
`sender`-capable `AcpSession`. A queued prompt's failed `session.prompt`
diagnoses through the existing `recorder().error` path, exactly like today's
`send_panel_prompt` failure handling, so the conversation shows the failure
under the prompt it belongs to and the queue continues on the next turn end.

### Queued prompts are a distinct presentation of a user message
`PanelMessage::User` becomes `PanelMessage::User { text, queued }` (one
variant, one flag) rather than a parallel variant: the bubble, the blue
tint, and the copy control all apply to both, and "queued" is exactly the
delta. While `queued`, the bubble renders muted with a "Queued" tag; enqueue
labels the message, delivery flips the flag. This keeps the copy-prompt
behavior (and its future siblings) for queued prompts by construction.

### The permission gate is checked at delivery, not at enqueue
Enqueuing must be possible up to the instant a permission prompt lands, and
delivery must not start while one is pending. Checking at delivery (in the
flush, and in `send_panel_prompt`'s immediate path) gives both: the queue never
clears over an unanswered prompt. If a permission request is pending at a turn
end, the promotion is deferred until the request resolves - the queued prompt
stays marked queued in the meantime.

### The send path loosens its gate
`send_panel_prompt` stops requiring `!turn_active`: it enqueues (recording the
message as queued, clearing the input and the pending context as today)
instead of restoring context and bailing. The permission check stays. The
send control's `can_send` follows suit (`!blocked && !empty`), keeping the
control live through a turn.

## Risks / Trade-offs

- [Flush races in the drain task (double-send or missed send)] → The one-shot
  `pending_delivery` flag, consumed inside the same task that applied the
  TurnEnd, serializes promotion to a single `session.prompt`; the queue head
  is removed only on promotion, so nothing is lost if a send errors.
- [Unbounded queue lets the user stack work the agent never invited] → Accepted
  for v1: the queue is a user-controlled queue, and a cap would otherwise
  have to reject - the very behavior this change removes. A cap is a future
  concern with no spec-level placement today.
- [Queued flag duplicates the delivered/delivery semantics] → The flag is the
  only difference between state the agent has seen and state it has not; it
  is single-sourced in `PanelState` and the renderer never invents it.