# Proposal

## Why

The panel does not show how much of an agent session's context window is used.
Users need this before a conversation reaches its effective limit.

## What Changes

- Decode ACP `usage_update` notifications and retain the reported used and
  total context tokens in the panel session state.
- Add a compact radial progress indicator to the panel input area's bottom
  bar. Its fill represents used context; the remainder represents available
  context.
- Show used and total token counts in its tooltip. Hide the indicator when an
  agent has not reported a meaningful context window size.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `acp-panel-ui`: the input area gains a radial context-window usage indicator.

## Impact

- `knot-acp` protocol parser, `knot` panel state, and workspace input bar.
- User-facing strings go through `knot_core::l10n::t`.
