# Proposal

## Why

The panel's attached context is only visible while there is some: the file and
image chips appear above the prompt box and vanish the moment the list is
empty, and each can be removed only one chip at a time. The input area's
bottom bar - the control row holding the send hint and the permission model
selectors - never says what the next prompt will carry, so the user has to
read the chips back into their head before hitting Send, and dropping a whole
screenshot set means four close-clicks.

## What Changes

- Add a **context indicator** to the panel input area's bottom bar: a compact,
  always-visible summary of the files and images attached to the pending
  message, counting files and images separately.
- Add a **clear-all** control on the indicator that detaches every attached
  item for that agent at once (today only per-chip removal exists).
- The indicator keeps a fixed place in the bottom bar whether or not anything
  is attached, so the row's other controls do not shift.
- The indicator's tooltip lists the attached names; the zero state reads as
  "no context".

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `acp-panel-ui`: the input area gains an always-visible attached-context
  summary and a clear-all control, complementing the existing per-item chips
  and the existing add-context control.

## Impact

- `crates/knot` workspace window: `render_panel_input_area` and the state it
  already holds (`panel_pending_context` per panel agent). No state grows and
  no crate below the UI changes; the indicator is derived entirely from the
  pending-context list the input area already renders.
- User-facing strings go through `knot_core::l10n::t`.