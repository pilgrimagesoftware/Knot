# Proposal

## Why

A queued prompt is often nearly right - a typo, a stale path, a changed mind - but the only
remedy is to delete it and type the whole thing again. Deleting throws away text the user
already wrote, which is the part worth keeping.

## What Changes

- Add an edit action to each queued message.
- Activating it SHALL remove the message from the queue and place its text in the panel
  composer, where the existing input, attachment, and send affordances apply.
- Sending the edited text SHALL enqueue it as a new message at the back of the queue.
- Offer edit only for a message that has not started delivery.
- Confirm before replacing composer text the user has already typed.
- Render edit as an icon button with a localized tooltip and accessibility label, matching the
  panel's other row controls.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `queued-message-management`: add an edit action that returns a queued message to the composer.

## Impact

- ACP panel queued-row actions and the panel composer's input state.
- Localization for the edit tooltip and accessibility label.
- Depends on `delete-queued-messages-and-icon-retry`, which introduces the
  `queued-message-management` capability and the stable queue identity this change addresses
  messages by. That change must be archived before this one's spec delta has a base to modify.
- No protocol or dependency changes.

## Non-goals

- Editing a message in place within its row.
- Preserving a message's queue position across an edit.
- Editing a message that is already in flight or delivered.
- Bulk editing, or editing attached context rather than prompt text.
