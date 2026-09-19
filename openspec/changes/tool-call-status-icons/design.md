# Design

## Context

See proposal.md - Why. The mechanics this rests on:

- `render_tool_call_card` renders the header as: disclosure chevron, kind
  icon, monospace title (flex-1, ellipsizing), then a status label
  (`status_label(&card.status)`) in the proportional family, muted or danger.
- `card_outline(status)` already classifies `pending`/`in_progress` →
  Info, `failed` → Danger, everything else → Neutral, and
  `PanelStyle::outline_color` turns that into the theme's
  `info_color`/`danger_color`/`border_color`. This is the exact color
  decision the icon needs - no new mapping.
- The bundled icon set offers `CircleCheck`, `CircleX`, `Loader`,
  `LoaderCircle` and `TriangleAlert`, which map one-to-one onto the statuses
  this app emits.
- `status_label` already turns a status into a word ("Pending", "Running…",
  "Done", "Failed") and passes unknown values through - identical to the
  tooltip and to the unknown-status behavior the spec keeps.

## Goals / Non-Goals

**Goals:**

- Room the title deserves back, without losing the state at a glance: one
  colored glyph where a word sat.
- One color decision and one vocabulary, both pre-existing, so the renderer
  merely composes them.
- The word survives as a tooltip, so the state does not turn into icon-only
  guessing.

**Non-Goals:**

- No new icons: the change reuses bundled glyphs.
- No animation (an animated spinner is a separate accessibility decision).
- No change to the title, kind icon, or disclosure chevron.
- No change to the card outline itself - the icon mirrors it, it does not
  replace it.

## Decisions

### Recognized statuses are an icon; unknown statuses stay inline text
The mapping is `pending`/`in_progress` → `Loader` (info), `completed` →
`CircleCheck` (neutral muted), `failed` → `CircleX` (danger). A status the
map cannot classify renders as the raw string, exactly as it does today. This
preserves the panel's stated philosophy that an unknown status "is more useful
shown than hidden" while giving every status this build can actually emit a
glyph. The alternative - a neutral fallback icon for unknowns - would imply a
readable state where the panel has none.

### The icon takes the card's own outline colour
`card_outline` already encodes "info while running, danger on failure,
neutral when done", and the card border uses it. The icon uses the same
`PanelStyle` colours, so status and outline can never disagree and a future
theme change recolors both at once. The only new colour is "completed": the
existing `MUTED` chrome colour already used for the kind icon and title.

### The word lives in the icon's tooltip, via `status_label`
Reuse `status_label` unchanged for the tooltip - no second vocabulary to keep
in step, and the unknown-status pass-through applies to the tooltip the same
way it does to today's inline text. The tooltip also restores the
learnability the icon trades away for space; hover and the card's own border
both carry the state.

### The header keeps its element id and hit target
The whole header remains the collapse toggle's target; the status icon is not
a second control and carries no click handler.

## Risks / Trade-offs

- [Icons alone are harder to learn than words] → Mitigated: the word is the
  tooltip, and the icon's colour matches the card border that already trained
  the user. This is the same glyph-and-word trade the kind icons already
  make.
- [The status icon and the kind icon read alike at small size] → The two
  already share a size class; the status icon sits at the row's far end
  (where the text sat) and is colored where the kind icon is muted, so
  position and colour separate them.
- [A spinner could be expected for "running"] → Explicit non-goal: animation
  is deferred, and the info colour plus "Running…" tooltip carry the state
  without it.