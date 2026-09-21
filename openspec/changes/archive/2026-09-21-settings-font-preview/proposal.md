# Proposal

## Why

The Appearance tab's Fonts section names three fonts and shows none of them.
Each row is a button reading `"Helvetica Neue, 13pt"` drawn in the UI font,
so the one thing a user picking a font wants to know - what it looks like - is
the one thing the control does not say. Answering it means opening the OS font
panel, which is the dialog the button exists to save you from.

The label is already the right place for the answer. It names a family and a
size and renders in neither.

## What Changes

- Each font picker's label SHALL render in the family it names, so the row
  shows the face rather than only spelling it.
- The label SHALL keep the settings window's own text size rather than the
  configured point size. The size is already stated as a number in the label;
  rendering at it would make a 48pt title font's row tall enough to push the
  section around and an 8pt one too small to judge.
- A family that does not resolve SHALL be marked as unavailable rather than
  drawn in a substitute face. A preview showing one face while naming another
  is worse than no preview: it reports a fault as a preference.

## Capabilities

### Modified Capabilities

- `settings-ui`: adds how the Appearance tab's font pickers render, and what
  they do when the persisted family cannot be resolved. The capability's own
  description of that tab was corrected separately, as baseline documentation
  of what already ships; this change assumes that correction and only adds.

### New Capabilities

(none)

## Impact

- `crates/knot`: `settings_window/mod.rs` - `font_picker_button` gains the
  family and the resolvability check, which needs the `App` it is not
  currently given.
- No change to `knot-core`, to what is persisted, or to how a font is chosen.
  This changes only how the current choice is displayed.

## Non-Goals

- Changing how a font is picked. The single button opening `NSFontPanel`
  stays; this is about what the button says while closed.
- A sample-text preview ("The quick brown fox…") beside each row. The label
  itself is enough to judge a face by and costs no layout.
- Repairing an unresolvable font, or offering to pick a replacement. Marking
  it is what this change owes the user; acting on it is a separate decision.
- Reconciling `settings-ui` against the shipped settings window. That was
  done separately as baseline documentation; this change proposes new
  behavior only.
