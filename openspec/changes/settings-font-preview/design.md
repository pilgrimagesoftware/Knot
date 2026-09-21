# Design

## Context

See proposal.md - Why. What the code already gives this change:

- `font_picker_button(id, target, name, size)`
  (`crates/knot/src/settings_window/mod.rs:1359`) builds the whole control:
  `Button::new(id).label(format!("{name}, {size:.0}pt"))` plus an `on_click`
  that opens `NSFontPanel`. All three rows in `render_appearance` (`:1367`)
  call it.
- `Button` implements `Styled`, so `.font_family(...)` is available on it
  directly; the label is rendered inside the button, so the family should
  inherit without touching `.label()`.
- The app already knows how to test a family for resolvability and already
  distrusts the persisted value: `terminal_font_family`
  (`workspace_window/mod.rs:98`) checks
  `cx.text_system().all_font_names()` and falls back to "JetBrains Mono"
  otherwise, with a comment naming exactly the failure modes this spec's
  second requirement covers - an old default, an uninstalled font, a value
  AppKit accepts that GPUI's lookup does not.
- The settings window re-renders on the font-panel poll
  (`settings_window/mod.rs:94-113`), which writes the new family into
  `self.settings`. So a preview read from `self.settings` at render time
  updates on the next frame with nothing added.

The one thing missing is access: `font_picker_button` is an associated
function taking no `App` or `Context`, and both the family lookup and the
theme need one.

## Goals / Non-Goals

**Goals:**

- The preview reads from the same persisted values the rest of the app reads,
  so it cannot disagree with what is in effect.
- Resolvability decided by the same question `terminal_font_family` asks, so
  the preview and the terminal cannot disagree about whether a font exists.

**Non-Goals:**

- Sharing a fallback with `terminal_font_family`. That function falls back to
  a monospace face because its caller is a terminal; the preview falls back to
  the window's own face. Same test, different answer, deliberately.
- Caching font names across renders. See the trade-off below.

## Decisions

### Pass `cx` into `font_picker_button` rather than resolving at the call site

`font_picker_button` gains an `&App` (or `&Context<Self>`) parameter and does
the resolvability check itself, returning a control that is already correct.

The alternative - resolving in `render_appearance` and passing a `bool` and a
family down - spreads one decision across four places and makes it possible
for a future fourth row to forget the check. The function already owns
everything else about how a font row looks.

This is the only signature change in the change.

### Test resolvability with `all_font_names`, matching `terminal_font_family`

The check is `cx.text_system().all_font_names().iter().any(|n| n == family)`,
which is what `terminal_font_family` does.

Alternative considered: `cx.text_system().resolve_font(...)` and comparing the
result. Rejected - `resolve_font` returns a `FontId` for anything, falling
back internally, so it answers "what will be drawn" and not "was the request
honored". The name list answers the question actually being asked.

### The unavailable marking is a suffix on the label, not an icon or a color

The label becomes `"{family}, {size}pt (unavailable)"` - or equivalent - drawn
in the default face.

Alternatives considered:

- *A warning icon in the row.* Needs a column the row does not have, and the
  Fonts group's rows are label-plus-control with nothing between.
- *Coloring the label.* Carries no meaning to a user who has not been told
  what the color means, and interacts badly with a button's own disabled and
  hover colors.

A word costs nothing and cannot be misread. It also survives the row being
read by accessibility tooling, which an icon or a color does not.

The exact wording is an implementation detail the spec deliberately leaves
open; the spec requires only that the row names the persisted family and is
marked.

### The preview does not repair the setting

The row goes on naming the persisted family and `ui_font_name` and friends are
not rewritten. This follows the precedent in `terminal_font_family`, which
falls back at the point of use and leaves the stored value alone - a font
missing today may be installed tomorrow.

## Risks / Trade-offs

- **`all_font_names()` is called once per row per render of the Appearance
  tab.** → It allocates a `Vec<String>` of every installed family, three times
  a frame, on a tab that redraws only on interaction. `terminal_font_family`
  already does the same per use, so this adds no new class of cost. If the tab
  feels slow, hoist one call in `render_appearance` into a set and pass it
  down - a local change that does not touch the spec. Measure before doing it.
- **`Styled` on `Button` may not reach the label.** → If the family does not
  inherit into the button's text, the control becomes a styled `div` child
  passed to the button instead of a `.label()`. Checked in task 1.1 before
  anything else is built; either way no requirement changes.
- **A family name from `NSFontPanel` may not be a name GPUI lists**, which is
  the exact case the second requirement exists for. This change makes that
  pre-existing fault visible for the first time, so a user may see
  "unavailable" on a font they believe they just picked. → That is the change
  working: the terminal already silently ignored such a choice. If it turns
  out to be common rather than rare, the fix is in the panel-to-name mapping,
  and this change is what makes that measurable.
- **Three faces in one small section can look untidy.** → Accepted; it is the
  information the section exists to convey.

## Open Questions

None.
