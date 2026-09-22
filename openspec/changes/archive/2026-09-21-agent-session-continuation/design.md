# Design

## Context

The idle delivery path injects a short inbox reminder into an agent session.
The session currently needs to distinguish that reminder from work-bearing
prompts so delivery cannot overwrite the active turn state.

## Goals / Non-Goals

**Goals:**

- Mark automatic inbox reminders as auxiliary events.
- Preserve the active prompt, turn state, and queued continuation while the
  reminder is handled.
- Resume the prior task after the reminder completes.

**Non-Goals:**

- No changes to message storage, unread flags, or retention.
- No replay of arbitrary user prompts.
- No persistence of interrupted session state across app restarts.

## Decisions

- Carry an explicit internal origin/type for the automatic inbox reminder
  rather than identifying it from its display text. Text matching is fragile
  and could misclassify a user's message.
- Route auxiliary reminders through the session's existing deferred-message
  mechanism, preserving the active turn and returning to it after the
  reminder is acknowledged. The reminder text should say: "Check your inbox.
  If there's nothing to do, continue your previous work." Replacing the
  active prompt would lose context.
- Keep normal messages on the existing queue path. Continuation is limited to
  the system-generated nudge so user intent and scheduling remain unchanged.
- Treat a reminder that arrives while no task is active as an ordinary inbox
  notification with no continuation state to restore.

## Risks / Trade-offs

- [A reminder can remain pending if the session never reaches a safe handoff]
  -> Keep the reminder unread/queued under the existing delivery rules and
  retry at the next eligible idle transition.
- [Restoring a stale turn could duplicate output]
  -> Resume only the session state that was active when the reminder was
  delivered, and preserve existing turn identifiers for deduplication.
