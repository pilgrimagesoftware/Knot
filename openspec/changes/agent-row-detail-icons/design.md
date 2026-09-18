## Context

See proposal.md — Why. How the row is built today, in `workspace_window`:

- The companion marker and the agent type each render as
  `h_flex().gap_1().items_center()` holding an `Icon::…xsmall()` in
  `muted_foreground` and a `text_xs` line.
- The persona renders as a plain `div` whose text is
  `format!("👤 {persona_name}")` - the icon is a character in the string.
- The status (`header_title`) and folder lines render as plain `div`s with no
  icon, at `text_size(ui_font_size)` rather than `text_xs` - so they are
  larger than the type and persona lines above them.

## Goals / Non-Goals

**Goals:**

- Four detail lines that each say what they are, in one aligned column.

**Non-Goals:**

- Changing which lines the row shows, or what they contain.
- Restyling the dashboard card, which shows some of the same fields and has
  its own layout question.
- Choosing new text sizes for the row. The existing size difference between
  the lines is left as it is; this change only stops the icons from fighting
  it.

## Decisions

### `Activity` for status, `Folder` for the directory

Both verified present in the generated `IconName` before being written down,
rather than recalled. The enum is generated at build time from the embedded
Lucide catalogue, so a plausible-sounding variant is not evidence of
anything - this project has already shipped a reference to an `IconName` that
did not exist, and the failure is a compile error at best and a blank icon at
worst.

`Activity` reads as "what this agent is doing"; `Folder` needs no
explanation. `Info`, `CircleDot`, `Signal` and `FolderOpen` were the other
candidates and all exist, so this is a taste call rather than a constrained
one.

### The persona's emoji becomes an icon, though it was not asked for

A drawn icon inherits the line's colour and size; an emoji does neither. Once
status and folder have muted, line-sized icons, the persona's full-colour
glyph is the one thing in the column that does not line up - so leaving it
would satisfy the request and still leave the row looking wrong.

`User` is the natural replacement and matches what the emoji was standing in
for. Called out in the proposal rather than slipped in.

### Each icon sizes to its line, rather than one size for the row

The type and persona lines are `text_xs`; the status and folder lines are the
UI font size. An icon fixed at `xsmall` across all four looks correct on the
first two and undersized on the last two.

The alternative - making every detail line the same text size - is a bigger
visual change than was asked for, and the existing hierarchy may well be
deliberate. So the icons follow the lines.

## Risks / Trade-offs

- **Four icons in a column is more furniture in a dense row** → they are muted
  and line-sized, which is what keeps them reading as labels rather than
  decoration; the alternative is four strings that have to be decoded.
- **`Activity` is a weaker signifier than `Folder`** → status is the harder
  thing to draw. If it reads poorly in use, the swap is one identifier, and
  the alternatives are recorded above.
- **The row already carries a state-coloured dot** → different job: the dot
  is the agent's machine state, the status line is what the agent says it is
  doing. They can disagree, which is why both exist.

## Open Questions

None.
