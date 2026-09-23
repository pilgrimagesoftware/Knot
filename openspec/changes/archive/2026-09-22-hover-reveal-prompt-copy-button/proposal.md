# Proposal

## Why

Every prompt in a conversation carries a copy button that is always drawn, so a
long exchange is a column of bubbles each with a permanent icon beside it. The
control is chrome competing with the content it sits next to, and it is there
for an action the user takes rarely.

The contract already said it should not be, and then lost the page it said it
on. The change that added the control
(`openspec/changes/archive/2026-09-20-panel-prompt-copy-button`) required it to
be "revealed on hover of the message", with a scenario titled "The control is
not persistent chrome" — and that delta never reached
`openspec/specs/acp-panel-ui/spec.md` when the change was archived. So the
control shipped always-drawn, and the requirement that would have caught it
does not exist in the specs. This change restores the requirement and makes the
code honor it; the hover behavior is not a new decision.

One thing does need deciding, and it is what the request makes explicit: what
counts as the hover region. The control sits *beside* the bubble, not inside it,
so a requirement worded as hover of "the message" and "the pointer leaves the
user message" would hide the button at the moment the pointer reaches it, making
it impossible to click.

## What Changes

- The prompt copy control is drawn only while the pointer is over that prompt —
  the bubble or the control itself, which together are one hover target.
- Revealing and hiding the control does not move anything. Its space is reserved
  whether it is shown or not, so a conversation does not reflow as the pointer
  travels down it.
- The hover target is the prompt cluster, not the full-width row the cluster is
  right-aligned within. Pointing at empty space to the left of a bubble reveals
  nothing.
- Nothing about copying changes: the same control, the same tooltip, the same
  clipboard text, the same confirmation.

Non-goals:

- The code block copy control. `acp-panel-ui` specifies it as always drawn, with
  a scenario named "The control does not wait for a hover", and that asymmetry
  is deliberate — a code block is a large target whose control sits inside it,
  while a prompt's control is chrome beside a one-line bubble. This change does
  not make them consistent, and a later change should not "fix" the difference
  without reading both requirements.
- The response action bar, which is a separate requirement and stays as it is.
- Introducing a hover-reveal convention for the rest of the app. This is one
  control.

## Capabilities

### Modified Capabilities

- `acp-panel-ui`: gains the "Prompt cell copy control" requirement. It is
  carried over from the archived change that was supposed to add it, plus the
  amendment this change decides — naming the hover region as the prompt and its
  control together, and requiring that revealing it does not reflow the
  conversation.

### New Capabilities

None.

## Impact

- `crates/knot/src/panel_view/message.rs` — the `PanelMessage::User` arm builds
  the row; the button and the bubble become a shrink-wrapped cluster that is the
  hover group, and the button is hidden outside it.
- No hover-reveal exists anywhere in `crates/knot` today, so this introduces the
  first use of the toolkit's group-hover styling. Keep it local to this control
  rather than generalizing it.
- No new dependency, no state, no new localization key — the existing
  `panel.copy_prompt` and `panel.copied_prompt` are unchanged.
- `openspec/specs/acp-panel-ui/spec.md` gains the requirement at archive time.

Found while checking that delta and **not fixed here**: two more requirements
from `openspec/changes/archive/2026-09-20-queued-message-appearance` —
"Queued messages use compact single-line typography" and "Failed status uses
failure color and proportional typography" — are also absent from
`openspec/specs/acp-panel-ui/spec.md`. Two changes archived on the same day both
lost their `acp-panel-ui` deltas, which suggests the archive step, not either
change. That is a different subject and belongs in its own change.
