# Design

## Context

ACP `session/update` notifications may carry `usage_update` with `used` and `size` token counts. Knot currently decodes no such update, so the panel has no context-window state.

## Decisions

### Keep usage in panel state

`knot-acp` decodes a typed usage update. `PanelState` stores the latest pair only when `size` is positive and clamps `used` to `size` for rendering. This is session-scoped and resets with the panel state.

### Render only with usable data

The bottom bar shows a compact radial indicator when usage exists. Filled segments mean used context and empty segments mean remaining context. A tooltip shows the exact used and total tokens. Agents that omit usage show no indicator.

### Preserve attachment behavior

The existing attachment chips and per-chip removal remain unchanged. They are unrelated to the model context-window budget.
