# Design

## Context

See proposal.md - Why. The relevant existing state and mechanics:

- The input area (`render_panel_input_area`) stacks, bottom to top: the
  control row (send hint on the left, permission/model/effort selectors and
  the expand control on the right), the prompt row (add-context button,
  textarea, Send), and the attached-context chips row when non-empty.
- `panel_pending_context: BTreeMap<Uuid, Vec<PathBuf>>` already holds every
  attached item per panel agent; the chips are rendered directly from it and
  removed one index at a time. No state needs to grow for this change.
- "bottom bar" names that control row, per the design decision "Control bar
  placement" that made it a sibling of the message list.
- The add-context control and the paste-image path already distinguish
  images from files ("Attach files or images"); the paste path detects image
  extensions when it writes a temp file.

## Goals / Non-Goals

**Goals:**

- Attached context is legible at a glance, including a fully empty pending
  message, without the user reading the chips row.
- One control clears a whole screenshot set at once.
- The change is derived state: nothing new is stored, nothing below the UI
  window changes.

**Non-Goals:**

- Not a workspace- or window-level status bar; it lives in the panel input
  area only.
- Does not show the agent's working folder, model, or token/character
  counts - those are the selectors' and sidebar's jobs.
- Does not change the chips row's per-item behavior.
- No persistence: the indicator follows the pending message and vanishes
  with it on send.

## Decisions

### The indicator lives on the left of the control row, beside the send hint
The left edge is the input area's quiet corner today (the hint is the least
dynamic control there), so the indicator reads as ambient state rather than
an action competing with the selectors on the right. It is a pill: a
paperclip icon, then the summary, then the clear-all control.

### The summary counts files and images as one string
A file/image split ("2 files · 1 image") at a glance beats a bare count,
which is the whole point of an indicator. The split needs a small
`is_image_path` helper over the already-attached `PathBuf`s; the paste path's
extension logic is the model, and the helper is unit-tested in isolation so
the indicator itself only formats.

### The indicator keeps its place when empty
A pill that appears and disappears shifts the hint and the selectors on every
attach/clear, which is exactly the layout instability `acp-panel-ui` is
written to avoid. The zero state is the same pill, muted, reading "no
context", with the clear-all control disabled.

### Clear-all is a control on the indicator, inoperative at zero
One click empties `panel_pending_context` for the agent's id. It is disabled
while the list is empty so it cannot silently read as "nothing happened" on a
misclick; at zero the "No context" pill already carries that meaning.

### Tooltip, not an expandable list
The attached names want to be one hover away, not a second row. Setting
`panel_pending_context`'s names on the indicator's tooltip keeps the tooltip
building entirely in the view, derived from the same list the chips render.

## Risks / Trade-offs

- [The indicator could read as a duplicate of the chips row] → The chips row
  carries identity and per-item removal; the indicator carries the aggregate
  and clear-all. They answer different questions, and the indicator is what
  survives an empty list.
- [A second image/file classifier drifts from the paste path] → The shared
  `is_image_path` helper is the single classifier; the paste path can adopt
  it or keep its own as long as the helper's tests pin the extension set.
- [The muted zero state may be mistaken for disabled] → The tooltip on the
  zero pill explains it ("no context attached"), and the clear-all next to it
  is the disabled control, not the pill itself.