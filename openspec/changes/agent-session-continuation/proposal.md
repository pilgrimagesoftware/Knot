# Proposal

## Why

An agent can receive the automatic "check your inbox" prompt while its prior work is paused or interrupted, leaving the agent to treat the nudge as a new task instead of resuming the in-flight session. The nudge needs explicit continuation semantics so interrupted work survives delivery of the inbox reminder.

## What Changes

- Queue the "check your inbox" prompt behind the active agent session instead of interrupting it.
- Update the prompt to tell the agent to continue its previous work if the inbox has nothing to do.
- Keep ordinary user prompts and unrelated agent messages on their existing queue behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mcp-messaging`: define continuation behavior for inbox nudges delivered around an active session.
- `queued-message-management`: drain a queued prompt for every agent rather than only the selected one, and identify a queued prompt the system generated.

## Impact

The agent session prompt/queue handling and MCP idle-nudge delivery path are affected. Message payloads and persistence do not change, and no dependency changes are required.

Two adjacent behaviors had to move with it. The queue drained only for the selected agent, which would have stranded a nudge queued for any other; and the queue is user-visible, so a nudge waiting in it needs to say it was not something the user typed.
