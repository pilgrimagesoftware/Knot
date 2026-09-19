# Tasks

## 1. Queue state

- [ ] 1.1 Add `queued_prompts: VecDeque<String>` and `enqueue_prompt`,
      `dequeue_for_delivery`, `queued_count` to `PanelState`, and update
      `PanelMessage::User` to carry a `queued` flag. Unit tests: enqueue
      order, dequeue FIFO, empty-queue dequeue returns None, and the
      `queued` flag defaults to delivered for the existing push path.
- [ ] 1.2 Extend the `TurnEnd` arm: pending permission defers promotion;
      otherwise promote the queue head (record the message, set
      `turn_active`/`tracking`, flip `queued=false`) and set the one-shot
      `pending_delivery`. Unit tests: promotion on turn end, deferral while
      a permission request is pending, and no promotion on an empty queue.
- [ ] 1.3 Add `take_pending_delivery` returning the owed prompt and clearing
      the one-shot. Unit test that it returns each value exactly once.

## 2. Delivery flush

- [ ] 2.1 In `panel_session`'s drain task, after applying each event, read
      `take_pending_delivery` and `session.prompt` the owed prompt, recording
      failures through `recorder().error` exactly like the existing send
      path. Verify with a state-level test that a turn end with a queued
      prompt drives one prompt call, and that a failed call leaves the next
      queued prompt deliverable.
- [ ] 2.2 Confirm the permission gate at delivery: a pending permission leaves
      the flag unissued, and once `answer_permission` resolves the queue's
      promotion and subsequent delivery proceed.

## 3. The composer

- [ ] 3.1 Loosen `send_panel_prompt`: drop the `!turn_active` requirement,
      keep the `pending_permission` gate, and enqueue the trimmed message
      (recording it as queued and clearing the input and pending context)
      when a turn is active. Verify in the app that sending mid-response
      clears the composer and marks the message queued.
- [ ] 3.2 Update `can_send` to `!blocked && !empty` so the Send control stays
      enabled through a turn. Verify the control enables mid-turn and still
      disables while a permission prompt is pending.
- [ ] 3.3 Render the `queued` presentation in `panel_view` (muted bubble with
      a "Queued" tag) and verify a queued prompt flips to a normal bubble
      when its turn starts.

## 4. Verification

- [ ] 4.1 Route the new user-facing strings ("Queued", any tooltips) through
      `knot_core::l10n::t`.
- [ ] 4.2 `make rust` passes clean (fmt, clippy, tests, build).
- [ ] 4.3 Exercise in the app: enqueue three prompts during one response,
      confirm they deliver in order with no overlap, one turn at a time.