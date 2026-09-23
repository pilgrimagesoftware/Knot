# Design

## Context

See proposal.md — Why. What the code and the toolkit already provide:

- The prompt row (`panel_view/message.rs`, the `PanelMessage::User` arm) is a
  single `h_flex().w_full().justify_end()` holding the button and then the
  bubble. The button is an unconditional child; there is no hover state anywhere
  in `panel_state` or `panel_view` to gate it on.
- GPUI supplies group hover directly: `.group(name)` on a container and
  `.group_hover(name, |style| ..)` on any descendant. `group_hover` resolves to
  the nearest ancestor group of that name, so one constant name is correct even
  with a group per message — there is no need to key it by index.
- GPUI's `Visibility` has exactly the semantics this needs: `Hidden` means "not
  drawn, but still takes up space in the layout". The `visible()` / `invisible()`
  helpers come from `visibility_style_methods!` on `Styled`.
- The conversation is virtualized through `ListState`, so rows are built and
  discarded as they scroll.

## Goals / Non-Goals

**Goals:**

- Hover handled by the toolkit's styling, with no hover state in `PanelState`
  and nothing new on the render path.
- Reveal that costs no layout.

**Non-Goals:**

- A reusable hover-reveal helper. One control uses this; a second one can
  extract the pattern when it exists.
- Any change to what the control copies or how it confirms.

## Decisions

### `visibility`, not opacity or conditional children

The button is always built and always laid out, and hover only toggles
`Visibility`. `invisible()` by default, `group_hover(GROUP, |style| style.visible())`
to reveal.

Alternatives considered:

- *Omit the child when not hovered.* Requires hover state to condition on —
  which means state in `PanelState` and a repaint per pointer move — and the
  button leaving the layout would shift the bubble sideways every time the
  pointer arrives. That is exactly the reflow the spec now forbids.
- *`opacity(0.)`.* Also avoids reflow, but leaves a fully transparent control
  that still hit-tests, and communicates "faded" rather than "not shown".
  `Visibility::Hidden` says what is meant and reserves the space by definition.

### The group is the prompt cluster, not the row

The existing `h_flex().w_full().justify_end()` stays as the row. A new inner
`h_flex()` — shrink-wrapped, holding the button and the bubble — carries
`.group(GROUP)`.

The row is full width, so making it the group would reveal the control from
anywhere on that line, including the empty space to the left of a short prompt.
That is the case the spec's "empty space beside a prompt reveals nothing"
scenario exists to rule out.

Because the button is inside the group, hovering the button is hovering the
group, so it stays visible under the pointer with no second rule. The spec
requires that; the structure provides it rather than a guard enforcing it.

### The 85% width cap moves to the cluster

The bubble's `max_w(relative(0.85))` has to move up to the cluster, and this
was found by shipping it the other way: a percentage resolves against its
containing block, and the cluster is shrink-wrapped, so a cap left on the
bubble resolved against an indefinite width and constrained nothing. Long
prompts grew past the row and read as left-aligned — the alignment regression
the first build produced.

On the cluster the percentage resolves against the row, which is `w_full` and
therefore definite. The cap now measures the button and the bubble together
rather than the bubble alone, so a maximum-width prompt is one button
narrower than it used to be. That is the accepted cost of keeping one cap;
widening the fraction to compensate would make the number mean less, not more.

### One constant group name

`group_hover` binds to the nearest ancestor carrying the name, and each message
builds its own group, so a single `&'static str` constant in
`panel_view/message.rs` gives per-message behavior. Keying it by index would
suggest the name is an identity when it is a lookup, and would add a
per-message allocation on the render path for nothing.

That the nearest-ancestor binding really does scope per message is the one
assumption worth checking in the running app rather than reading off the API:
a conversation with several prompts, hovering one, is the test.

## Risks / Trade-offs

- **A hover-only control is unreachable without a pointer.** This is the accepted
  cost of the existing requirement, not something this change introduces, and it
  is why `acp-panel-ui` deliberately specifies the *code block* copy control the
  other way. If prompt copying ever needs a keyboard route, that is a new
  requirement, not a reason to leave the control permanently drawn.
- **First use of group hover in `crates/knot`.** Nothing else in the binary
  reveals on hover, so there is no house pattern to match and no shared helper to
  break. Keep it in `message.rs`.
- **Virtualized rows.** Group hover is pure styling resolved during paint, with
  no state to carry across a row being recycled, so a row scrolling out and back
  cannot leave a control stuck shown. Worth confirming by scrolling a long
  conversation with the pointer held still over the list.
- **The reserved space is visible as a gap.** The bubble no longer sits flush to
  where the button was; the button's width is held open beside it at all times.
  This is the deliberate trade against reflow. If the gap reads badly, narrow the
  control or its padding — do not reclaim the space by removing it from layout.
