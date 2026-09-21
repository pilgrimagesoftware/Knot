# Design

## Context

See `proposal.md` - Why, for the defect and its cause. What the fix has to work
within:

- `render_tool_call_card` (`panel_view/tool_call.rs`) already branches on
  `collapsed`, applying `overflow_hidden().whitespace_nowrap().text_ellipsis()`
  in that branch and nothing in the other. The branch is the right shape; only
  the text going into it is wrong.
- The verified cause is in `gpui::text_system::shape_text`: it does
  `text.split('\n')` and shapes each piece as its own line, before any
  wrapping or truncation decision. `TextOverflow::Truncate` is then applied per
  shaped line. So `whitespace_nowrap` cannot make multi-line text one line, at
  any call site, and no combination of style flags will.
- The `knot` crate keeps small presentational text helpers in `app_support.rs`
  (`shorten_path` is there, with a comment tying it to its Swift counterpart).
- The repo's testing convention for render code is to pull the decision into a
  pure function and test that - `font_label`, `status_label`, `diff_lines` are
  all tested this way, with no window or text system.
- Seven call sites declare a single line today: the collapsed tool-call title,
  the queued-prompt row, `detail_line`, the workspace title bar's activity text
  and folder, the dashboard card's activity text and folder, and the Markdown
  pane's title.

## Goals / Non-Goals

**Goals:**

- One function, tested directly, that every single-line render passes its text
  through - so a future single-line row has an obvious thing to call and the
  reason is written down once.
- Leave every stored and delivered string untouched; this is a drawing concern.

**Non-Goals:**

- A general-purpose text-truncation utility (by graphemes, by width, by column
  count). The ellipsis is the renderer's job and works correctly once the text
  is one line.
- Changing the style flags at the affected sites. `whitespace_nowrap` and
  `text_ellipsis` are still needed - they handle the long-single-line case,
  which is the case they always handled.

## Decisions

### Flatten the text, do not fight the shaper

`single_line(text) -> String` splits on ASCII whitespace and rejoins with a
single space (`text.split_whitespace().collect::<Vec<_>>().join(" ")`), which
collapses newlines, tabs and runs of spaces and trims both ends in one pass.

Alternatives considered:

- Replacing only `\n` with a space. It leaves a heredoc's indentation in the
  middle of the line, so the visible part of a collapsed header is mostly
  whitespace - the screenshot's commands are indented eight columns.
- Truncating at the first newline. Discussed and rejected: the part of a wrapped
  command that identifies it is often past the first line break.
- Asking GPUI for a "shape as one line" mode. The behavior is correct for a
  text shaper; the fault is Knot handing arbitrary multi-line text to an element
  it has declared single-line.

`split_whitespace` is ASCII-plus-Unicode-whitespace aware and allocates one
`String`; it runs once per affected row per frame, on strings the size of a
command line. That is the same order of work as the `format!` calls already on
these paths.

### Flatten at the point of render, not at ingest

The tool-call title is flattened inside the `collapsed` branch, so the expanded
branch keeps the agent's text verbatim - which the requirement demands and which
the ADDED requirement generalizes ("The text itself SHALL NOT be altered").
Flattening in `ToolCallCard` on the way in would be a smaller diff and would
break the expanded card.

The same rule applies to the queued prompt: the row is flattened, `prompt.text`
is not, so delivery and the delivered message are unaffected.

### `detail_line` flattens for every size, not only the truncating one

`detail_line` renders Small lines wrapping and Body lines truncating. Flattening
only the Body branch would leave a Small line able to hard-break, so the helper
is applied to the text once, before the size branch. This does not defeat the
Small line's intended wrapping: wrapping is a soft-wrap decision at the width,
which a single long line still gets.

### The helper lives in `app_support.rs`

Beside `shorten_path`, for the same reason: it is presentational, it is used
from several modules of the `knot` crate, and nothing outside that crate needs
it. Putting it in `knot-core` would make a crate with no UI own a rendering
detail.

## Risks / Trade-offs

- [A collapsed header now hides where the command's line breaks were, so two
  visually similar commands read alike at a glance] → The card expands to the
  exact text, one click away, and the alternative is the current behavior where
  a collapsed card is indistinguishable from an expanded one.
- [`split_whitespace` also collapses runs of spaces inside a quoted string, so
  `grep "a  b"` reads as `grep "a b"` in a collapsed header] → Accepted: the
  header is an identifying summary, not a copyable command, and the expanded card
  is verbatim. Noted here because it is the one visible difference from replacing
  newlines alone.
- [Seven call sites means one can be missed, and a missed site fails only for
  input nobody produces during testing] → The implementation task list names each
  site, and verification uses text with a deliberate newline at each one rather
  than whatever the app happens to show.
- [A future single-line row is added without the helper and reintroduces the
  defect] → The helper's doc comment states the GPUI behavior and why the style
  flags are not enough, so the next reader finds the reason at the function they
  are looking for rather than in this change's history.
