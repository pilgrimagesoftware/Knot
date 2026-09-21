# Design

## Context

See proposal.md - Why. `delete-queued-messages-and-icon-retry` gives each queued message a stable
identity and its own row controls; this change adds one more control to that row and one path out
of the queue, back into the composer the panel already has.

## Goals / Non-Goals

**Goals:**

- Recover a queued message's text for editing without retyping it.
- Reuse the panel composer rather than growing a second text-entry surface.
- Keep the queue's remaining order untouched.
- Never discard text the user typed without asking.

**Non-Goals:**

- An inline editor within the queued row.
- Holding a message's queue position while it is edited.
- Editing a message already in flight, or editing its attached context.

## Decisions

### Edit returns the message to the composer

Edit removes the entry by stable id and sets the composer's value to its text. The composer
already owns attachments, expand/collapse, permission mode, and send, so an edited message
reaches the agent through exactly the path a new one does.

Alternative: an inline input in the row. Rejected because it would duplicate the composer's
behavior in a second place, and the row is a single truncated line by design - the place where
long prompt text is least workable.

### The message loses its queue position

An edited message is enqueued afresh when sent, behind whatever is already waiting. Nothing is
reserved for it.

Alternative: hold the slot until the edit is sent. Rejected because the queue would have to keep
a place for a message that may never be sent, and the panel would have to explain a gap that
only one user action can fill.

### Confirm before replacing typed text

When the composer is empty, edit fills it directly. When it holds text the user typed, a
confirmation dialog asks before replacing, matching the project's convention for actions that
destroy work. Declining changes nothing: the entry stays queued and the composer keeps its text.

Alternative: refuse edit while the composer is non-empty, or append to it. Rejected - the first
strands the message with no way forward, and the second silently splices two prompts together.

### In-flight messages expose no edit control

The control is absent, not disabled, for an entry whose delivery has started - the same rule the
delete control follows. A prompt the agent already has cannot be taken back by editing a row.

## Risks / Trade-offs

- [The user edits and never sends] -> The text sits in the composer, which is visible and behaves
  like anything else typed there; the message is not lost, only moved.
- [Edit races with the pump promoting that message] -> Both act on the stable id: the pump marks
  the entry in flight, which removes the control, and an edit arriving for an entry that is no
  longer editable is ignored rather than applied.
- [Confirmation fatigue on every edit] -> The prompt appears only when the composer actually
  holds typed text, which is the case where something would otherwise be lost.

## Migration Plan

No data migration is required. The change is additive: without it, queued messages can still be
deleted and retyped. Rolling back removes the edit control and leaves the queue and composer
behaving as they did.
