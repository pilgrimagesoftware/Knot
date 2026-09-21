# Proposal

## Why

Queued messages currently use the same presentation as ordinary message content, which makes long queued text consume vertical space and makes failed status text blend into surrounding content. The queue row needs a compact, recognizable treatment so users can scan message state quickly.

## What Changes

- Render queued message content in a single-line monospace row with ellipsis truncation.
- Render the `failed` status in the theme's failure color using the proportional font.
- Keep expanded or non-queued message content unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `acp-panel-ui`: clarify typography and truncation for queued message rows and failed status text.

## Impact

The ACP panel message-row rendering and its theme typography/color lookups are affected. No protocol, persistence, or dependency changes are required.
