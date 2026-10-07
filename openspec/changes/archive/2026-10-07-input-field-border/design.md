# Design

## Context

gpui-kit's `Input` draws `border_1().border_color(theme.input)` when
unfocused, then applies the caller's `StyleRefinement` (`refine_style`), then
the focus ring when focused. On macOS, Knot's `apply_system_palette` sets
`theme.input` to `controlBackgroundColor`, which is close to the field's fill
and the toolbar's `background`. `theme.border` is `separatorColor`.

## Goals / Non-Goals

**Goals:**
- A visible unfocused border on the Pull Requests search field only.

**Non-Goals:**
- Touching the global theme; see proposal.md - Non-Goals.

## Decisions

### Override the border color on the one field

`search_field` takes the border color and calls
`.border_color(cx.theme().border)` on its `Input`. Because the caller's style
is applied after `Input`'s own border and before its focus ring, the override
replaces only the unfocused border; focus still paints the accent ring.
`theme.border` is `separatorColor` on macOS and the shipped theme's border
elsewhere, so the field follows appearance changes with the rest of the
chrome.

Alternatives:
- Remap `theme.input` globally. Rejected: it changes every input control,
  which is not wanted.
- Wrap the field in a bordered `div`. Rejected: two borders stack when
  focused, and the wrapper's radius has to be kept in step with `Input`'s.

## Risks / Trade-offs

- [A gpui-kit update reorders `refine_style` after the focus ring, so the
  override hides the focus border] → The manual check covers focus; a
  gpui-kit bump that changes it shows up there.
- [`separatorColor` is translucent and may be faint in light appearance] →
  Manual check; if faint, use `border` composited opaque over `background`.
