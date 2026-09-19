# Design

## Context

See proposal.md - Why. The mechanics this rests on:

- `render_tool_call_card`'s header is one `h_flex` row: disclosure chevron,
  kind icon, title (`flex_1().min_w_0()` in the mono family, no nowrap),
  and the status indicator (`flex_shrink_0`).
- The title is the only flexible child, so its `min_w_0` lets the row stay
  inside the pane; but without `whitespace_nowrap` a long title wraps inside
  its flex box, growing the collapsed card past one line.
- The codebase already has the exact ellipsizing pattern this needs:
  `detail_line` renders one-line rows with
  `overflow_hidden().whitespace_nowrap().text_ellipsis()`.
- `collapsed` is already computed per card and passed into
  `render_tool_call_card`, so no state or signature changes are needed.

## Goals / Non-Goals

**Goals:**

- Collapsed height is a constant: one line, whatever the title.
- The status indicator is never sacrificed to make room for an ellipsized
  title.

**Non-Goals:**

- No behavior change to expanded cards.
- No change to the collapse rules, the disclosure toggle, the kind icon, or
  the status indicator.
- No hover/reveal of the full title (the tooltip already carries it).

## Decisions

### Ellipsizing applies to the title element, conditionally on `collapsed`
When `collapsed` is true, the title div gains
`whitespace_nowrap().overflow_hidden().text_ellipsis()` on top of its existing
`flex_1().min_w_0()`. `flex_1` lets the title absorb the width change when
the pane resizes; `min_w_0` already prevents it from stretching the pane; the
new properties make it consume its line and truncate. Expanded cards keep the
current styling verbatim.

### The status indicator is not given up to the ellipsis
`flex_shrink_0` on the indicator means the ellipsis budget comes entirely out
of the title, and gpui lays the row out accordingly. This is what keeps the
second scenario ("indicator stays on the line") true by construction rather
than by tuning.

### The tooltip already covers discoverability
The full title is available when needed - the title element already carries
no truncation tooltip today, but the `detail_line` precedent proves the
ellipsis pattern, and nothing here removes information the user can already
reach. No new title-reveal affordance is added.

## Risks / Trade-offs

- [A very short pane leaves almost no room for the title next to the
  indicators] → Acceptable: the collapse toggle, kind icon and status stay,
  and the title yields first by design. The tooltip preserves the text.
- [Applying nowrap only to the title could still allow a long *status word*
  to wrap] → The status element is `flex_shrink_0` text whose width comes
  from its own nowrap-free content; as a `div` with `flex_shrink_0` it cannot
  wrap the line unless it exceeds the pane alone. This is the pre-existing
  behavior and is unchanged by this change.
- [The in-flight `tool-call-status-icons` change touches the same header
  row] → The two changes touch different children (title vs status element)
  and are independent; whichever lands first, this change only adds layout
  properties to the title while `collapsed`.