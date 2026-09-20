# Proposal

## Why

An agent can receive the automatic "check your inbox" prompt while its prior work is paused or interrupted, leaving the agent to treat the nudge as a new task instead of resuming the in-flight session. The nudge needs explicit continuation semantics so interrupted work survives delivery of the inbox reminder.

## What Changes

- Preserve the active agent session's pending work when a "check your inbox" nudge is delivered.
- Resume the interrupted task after inbox handling rather than replacing it with the nudge.
- Keep ordinary user prompts and unrelated agent messages on their existing queue behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mcp-messaging`: define continuation behavior for inbox nudges delivered around an active session.

## Impact

The agent session prompt/queue handling and MCP idle-nudge delivery path are affected. Message payloads and persistence do not change, and no dependency changes are required.
