# Design

## Context

See proposal.md - Why. The mechanics this rests on:

- `PanelMessage::User(text)` renders as a right-justified, blue-tinted bubble
  in `render_message`'s `User` arm, with no actions of its own.
- Completed assistant responses get `render_response_actions`, whose copy
  button (`panel-copy-response`) performs
  `cx.write_to_clipboard(ClipboardItem::new_string(text.clone()))` - the
  existing, verified clipboard path this change reuses.
- `render_message` already receives an `index` and the `PanelMessage` text,
  so the copy needs nothing new from `PanelState`.

## Goals / Non-Goals

**Goals:**

- Symmetric copy affordances: a prompt is as one-click portable as the
  response that answered it.
- The bubble's visual identity is untouched when idle.

**Non-Goals:**

- No scroll-to controls for prompts (the response action bar's other two
  buttons answer response-side questions; a prompt is already the source the
  response scrolls back to).
- No copy for system/tool content or error messages.
- No edit or re-send affordances.
- No change to what a *response* copies.

## Decisions

### Hover-revealed, not always visible
User bubbles are expected, numerous, and unbounded in width; a permanent
action bar would put chrome on every line of a long exchange. The panel's
established pattern is to keep small controls out of the resting layout (the
response actions appear only once a response completes, the track toggle
replaces them while it streams), so the prompt copy follows it: a ghost button
revealed on hover, hidden on leave. The spec's "not persistent chrome"
scenario pins this.

### Placed left of the bubble, outside the tinted cell
The bubble is right-justified, so the natural free space is its left edge.
The control sits on the message row immediately left of the bubble, aligned
with its top line - outside the blue fill, so it cannot fight the bubble's
edge or look like part of what it copies.

### The hover target is the whole message row, not the bubble rect
A wrapped long prompt is a tall target; measuring its exact painted bubble to
arm the control is fragile. The row is the hover surface, which keeps the
control predictable for short and multi-line prompts alike.

### It copies the `text` field exactly
The user message holds a plain `SharedString`; the bubble's chrome and the
session's attachments are separate state and are not part of that text. Copying
`text` verbatim matches the response-side copy contract and satisfies the
"attachments are not part of the copy" scenario by construction.

## Risks / Trade-offs

- [Hover-revealed controls are invisible to the discoverability-impaired] →
  Mitigated by the mouse being the only way a prompt bubble is currently
  interacted with, and by the same pattern already setting precedent on the
  response side; a keyboard path can arrive with the panel's larger
  accessibility work.
- [Right-aligned bubble + row-wide hover could arm the control while the
  pointer is far from it] → The button still paints only beside the bubble;
  the wide hover surface widens the *target*, and the button's own position
  keeps the affordance honest.