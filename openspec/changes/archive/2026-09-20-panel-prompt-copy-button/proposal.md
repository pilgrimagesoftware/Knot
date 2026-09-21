# Proposal

## Why

Only the agent's response is copyable today; the prompt that produced it is
not. Re-pinning a past question means hand-selecting text inside a
right-aligned, tinted bubble that can wrap - exactly the kind of selection a
chat UI should never make the user do by hand. The response side already has
a copy control; the prompt side should answer the same question symmetrically.

## What Changes

- Add a **copy control** to each user message (the prompt bubble): revealed on
  hover, it copies the prompt's text to the clipboard.
- The copied content is exactly the prompt text - not the surrounding bubble
  chrome and never any attached-context payload.
- The control mirrors the assistant response's copy button
  (`panel-copy-response`), making the two sides of the exchange equally
  portable.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `acp-panel-ui`: a prompt cell gains a copy control, so the panel's
  clipboard story covers user messages as well as responses.

## Impact

- `crates/knot` panel view: the `PanelMessage::User` arm of `render_message`
  gains the hover-revealed control; the clipboard write reuses the
  `write_to_clipboard` pattern `panel-copy-response` already uses.
- No state changes and no crate below the UI window changes.
- User-facing strings go through `knot_core::l10n::t`.