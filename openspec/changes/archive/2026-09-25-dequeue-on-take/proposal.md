# Proposal

## Why

When a queued prompt is handed to its agent, its row stays in the queue,
marked in flight, until the agent's turn ends. The prompt is already in the
conversation by then, so the queue shows it twice for the length of the turn
and reads as if it were still waiting.

## What Changes

- A queued prompt leaves the queue when the pump takes it, not when its turn
  finishes.
- A prompt whose delivery fails returns to the head of the queue, marked
  failed, so the user can still retry or delete it.
- The in-flight queue row, and the rule that it cannot be edited, go away:
  no queued row is ever in flight.

Issue: https://github.com/pilgrimagesoftware/Knot/issues/485

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `queued-message-management`: delivery removes the entry when it starts;
  a failed delivery puts it back at the head.

## Impact

- `knot`: `prompt_queue` loses `in_flight`; the pump moves the taken prompt
  into a per-agent in-flight slot, which blocks the next prompt until the
  result arrives and restores the prompt on failure.
