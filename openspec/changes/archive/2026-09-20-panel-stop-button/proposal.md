# Proposal

## Why

Once an ACP agent starts a turn, the panel gives the user no direct way to stop work that is taking too long, has gone in the wrong direction, or is no longer needed. A visible stop control makes interruption available where the active work is shown.

## What Changes

- Add a stop control to the panel input area while an ACP turn is active.
- Send the ACP cancellation request for the active session when the control is activated.
- Clear the active-turn state when cancellation succeeds or the turn finishes, so the stop control disappears and normal input returns.
- Keep cancellation scoped to the selected panel session and make repeated activation harmless while cancellation is in flight.
- Show a localized accessible label and tooltip for the stop control.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `acp-panel-ui`: Add an interrupt control for an in-progress ACP turn and define its cancellation behavior.

## Impact

- `crates/knot/src/panel_view/` input controls and active-turn state.
- `crates/knot-acp/` or the existing ACP session adapter, if cancellation is not currently exposed to the panel.
- Localization entries and panel UI tests.
- No terminal-process cancellation or cross-session cancellation is included.
