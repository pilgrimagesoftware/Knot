# Tasks

## 1. Model auxiliary inbox nudges

- [x] 1.1 Locate the idle delivery and session prompt queue paths, then add an explicit origin for automatic inbox nudges; verify normal user messages still use the existing origin.
- [x] 1.2 Queue the automatic inbox nudge behind an active turn instead of interrupting it; verify the active state remains unchanged while the nudge waits.
- [x] 1.3 Update the nudge text to tell the agent to continue its previous work if there is nothing to do; verify the exact fallback instruction is delivered.
- [x] 1.4 Resume the preserved task after the nudge is acknowledged; verify the task continues with its original turn identity and does not duplicate output.

## 2. Tests and verification

- [x] 2.1 Add focused tests for a nudge during paused work, a nudge during an active update, and a nudge with no active task; verify each expected queue transition.
- [x] 2.2 Run the affected MCP/session tests and verify all pass.
- [x] 2.3 Run repository formatting and broader test checks required by the touched crate and verify no regressions.

## Notes

- `send_inbox_nudge` no longer prompts the session directly; it delegates to
  `deliver_panel_prompt` with `PromptOrigin::InboxNudge`, so a nudge that
  arrives after a turn has started queues instead of prompting over it.
- `NudgeCheck::can_receive` was removed. Session readiness gated the
  *decision*, so a nudge that failed it was dropped and left to a later poll;
  the delivery path queues now, so the decision is only about the message and
  the agent. `panel_can_take_a_prompt` went with it.
- A message is recorded in `nudged_messages` only when the nudge was taken
  (sent or queued), preserving "Recipient with no live session".
- `panel_needs_repaint` drained only `selected_agent`'s queue, so a queued
  prompt behind a background agent's turn waited for the user to click that
  agent. Nudges queue precisely for agents nobody is looking at, so this now
  drains every agent with a non-empty queue.
- Queued nudges are visible in the user's own prompt queue, so the row's
  accessible name says where the prompt came from (`panel.queued_inbox_nudge`).

### Test coverage limits

There is no harness for a full `WorkspaceWindow` with a fake panel session,
so the queue transitions are covered at the layers that are testable:
`prompt_queue` (ordering, origin, promotion after the entry ahead completes),
`app_state` (the nudge decision), and the l10n catalogue (the row's label).
The `deliver_panel_prompt` readiness branch itself remains uncovered, as it
was before this change.
