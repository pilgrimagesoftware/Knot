## Context

See proposal.md — Why. What the card is today, in `panel_view`:

- `render_tool_call_card(card, style)` draws a header of icon, title and
  status, then the content blocks. It takes no render context, so it cannot
  reach the theme - which is why its colours are file-level constants
  (`CARD_BG`, `CARD_BORDER`, `ERROR_COLOR`, `MUTED`).
- `PanelStyle` already solves that problem once, for the monospace family:
  the caller reads the theme and passes the value down. Its field carries a
  comment recording that `font_family("monospace")` is not a family GPUI
  resolves and silently fell back to the body font.
- `status_label` already maps the wire status to "Pending", "Running…",
  "Done" and "Failed", and `ToolCallCard::failed()` / `is_finished()` already
  separate the three states this change colours.

## Goals / Non-Goals

**Goals:**

- A card whose state is readable without reading it.
- Theme colours, not constants, for anything this change touches.

**Non-Goals:**

- Restyling the rest of the panel. The permission prompt, the diff view and
  the message bubbles keep their current colours, including their own
  hardcoded ones; sweeping those into the theme is worth doing and is not
  this change.
- New states. Pending and running are coloured alike because the card cannot
  usefully distinguish "about to start" from "started" in an outline.
- Changing the status *words*. "Done" stays "Done".

## Decisions

### `danger`, not `warning`

The request offered "the danger/warning color". They are separate theme
tokens and the distinction is real: warning is a caution about something that
might go wrong, danger is something that did. A failed tool call is the
second, and `cx.theme().danger` is already what this project uses for error
text and what its UI conventions name.

### `info` for in-progress, rather than a blue

"Outlined in blue" names an appearance; `info` names the role and happens to
be blue in this theme. Taking the token means the card follows a theme that
chooses a different blue - or a different hue entirely - instead of sitting
at a fixed `0x3B82F6` while everything around it moves.

### The theme reaches the card through `PanelStyle`, not a new context

The card is rendered from a plain function with no `cx`, by design - the
panel is rendered as data. Threading a render context into it to read three
colours would undo that for the sake of three colours.

`PanelStyle` is the established answer and already carries a theme value for
exactly this reason. It gains the three colours, read once at the call site
where a context exists.

*Alternative considered:* give `render_tool_call_card` a `&Context`.
Rejected - it spreads render context through the panel's pure rendering
helpers, and the precedent set here would be followed by the next colour.

### Completed keeps the neutral border, and that is the point

The temptation is symmetry: red for failed, blue for running, green for
succeeded. The request rules it out and is right to - an outline is a way of
saying *look here*, and saying it about every successful call says nothing.
Written into the spec as a requirement rather than left as an omission, so a
later "finish the set" does not quietly add green.

### Two fonts in one header

Splitting the header's fonts is the change that does the most work for the
least code: monospace marks what the agent produced, proportional marks what
the panel is saying about it. The requirement pins both halves, including the
half that may already be correct today, because the failure mode is a future
change setting the whole header monospace and swallowing the distinction.

One trade-off: a `think`-kind call's title is prose, and it will render
monospace along with the commands and paths. Accepted - titles are
overwhelmingly commands, paths and identifiers, and a per-kind font rule
would be more surprising than a consistent one.

## Risks / Trade-offs

- **`info` may not be blue in every theme** → that is the intent, not a
  defect: the role is "this is in progress", and a theme that renders roles
  differently should render this one differently too.
- **A running call's outline draws the eye during a long turn** → it should;
  it is the one thing happening. It stops when the call completes.
- **This change and `collapse-finished-tool-calls` both edit the same header**
  → they compose without conflict (a control versus fonts and colours), but
  whichever lands second rebases onto the first.

## Open Questions

None.
