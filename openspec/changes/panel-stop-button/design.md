# Design

## Context

The panel already tracks whether an ACP response is active and exposes send-state controls in the input area. ACP session handles own the turn lifecycle, so cancellation must use the existing session boundary rather than introducing a process-wide interrupt path.

## Goals / Non-Goals

**Goals:**

- Expose one stop action for the selected active ACP session.
- Reuse the existing turn lifecycle and completion notifications to reset the control.
- Keep cancellation idempotent while a request is pending.
- Cover the control visibility, session scoping, and terminal-state reset with tests.

**Non-Goals:**

- Stopping terminal processes.
- Cancelling another panel's session.
- Adding a second response state machine solely for the button.

## Decisions

- Put the control beside the existing send control and swap the active-turn action to stop when the session cannot accept concurrent input. This keeps the interaction in the input area and avoids adding a separate toolbar.
- Add cancellation at the existing ACP client/session abstraction if it is missing, then call it from the panel. This keeps transport details out of the view and makes session scoping explicit.
- Treat the first stop activation as the only effective request until the session reports completion. A local in-flight guard prevents duplicate cancellation requests without changing the ACP protocol.
- Use the existing localization and button styling conventions for the label, tooltip, and accessibility text.

## Risks / Trade-offs

- [ACP adapter does not support cancellation uniformly] -> Make the session capability explicit and keep the stop control hidden or disabled for adapters that cannot cancel, rather than pretending cancellation succeeded.
- [Cancellation races with normal completion] -> Accept either event ordering and clear state from the shared terminal-state path.
- [Agent continues emitting late events] -> Preserve existing event handling and mark the turn terminal before accepting a new prompt.

## Migration Plan

No data migration is required. Add the cancellation method and UI behavior behind the existing panel flow, then verify normal completion and cancellation paths.
