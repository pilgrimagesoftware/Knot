# Proposal

## Why

Queued prompts can become stale before the agent processes them, but users currently have no way to remove one. The retry action also breaks the panel's established icon-plus-tooltip control convention by rendering as text.

## What Changes

- Allow users to delete an individual queued message before it is sent.
- Remove the message from the queue immediately and leave already-running work unchanged.
- Render retry as an icon button with an accessible tooltip instead of a text button.
- Use the existing icon-button convention for the retry control without changing retry behavior.

## Capabilities

### New Capabilities

- `queued-message-management`: Delete individual messages waiting to be sent.

### Modified Capabilities

- `acp-panel-ui`: Add queued-message deletion and change retry presentation to an icon with tooltip.

## Impact

- ACP panel queue state and message-row actions.
- Retry control rendering, accessibility labels, and localization.
- No protocol or dependency changes.

## Non-goals

- Cancelling a turn that has already started.
- Editing queued message content.
- Changing retry semantics or adding bulk queue management.
