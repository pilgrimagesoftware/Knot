# Proposal

## Why

A collapsed tool call is supposed to fold to one ellipsized line
(`acp-panel-ui` - "Collapsed tool call header is a single line"), and it does
not: a call whose title is a multi-line shell command renders every line of that
command, so a collapsed card fills most of the pane and reads exactly like an
expanded one.

The header already sets `whitespace_nowrap` and `text_ellipsis`, which is why
this looked implemented. GPUI's text shaper splits its input on `\n`
unconditionally (`shape_text` in `gpui::text_system`); `WhiteSpace::Nowrap` only
disables *soft* wrapping, and the truncation then applies per shaped line rather
than to the block. Any element that declares a single line renders as many lines
as its text has newlines.

## What Changes

- Add one helper that collapses a string to a single line: every run of
  whitespace - newlines, tabs, repeated spaces - becomes a single space, and the
  result is trimmed. Applied where a render declares one line, it makes the
  declaration true; the width-based ellipsis then works as intended.
- Fix the reported defect: a collapsed tool call's title is flattened before
  rendering, so the header is one line ending in an ellipsis. An expanded card
  keeps the title exactly as the agent sent it, newlines included.
- Fix the same defect at the other sites that declare a single-line render of
  text Knot does not control. Each is a one-call change:
  - the queued-prompt row in the panel's input area - a user's multi-line prompt
    currently renders one line per newline while it waits;
  - the sidebar agent row's detail lines (`detail_line`), whose comment already
    states the intent - "truncate rather than wrap, which is what keeps a long
    path from growing the row";
  - the workspace title bar's activity text and folder;
  - the dashboard card's activity text and folder;
  - the Markdown pane's title.
- An agent can set multi-line status text through the `set-status` MCP tool and
  a POSIX path may itself contain a newline, so none of those sites is
  hypothetical - they are the same defect waiting for the same input.

### Non-goals

- Changing what an expanded tool call shows. A multi-line title is legitimate
  content there and stays as it is.
- Truncating by character count, adding a "first line only" rule, or marking
  that a title had more lines. The joined line ellipsizes at the width it has,
  which is what the existing requirement asks for.
- Touching `diff_stats_row`, the one other `whitespace_nowrap` site: it renders
  numbers Knot formats itself and cannot receive a newline.
- Reporting the GPUI behavior upstream as a bug. Splitting on `\n` is what a
  text shaper should do; the fault is Knot asking an element to render arbitrary
  multi-line text as one line.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `acp-panel-ui`: the collapsed-header requirement states what happens to a
  title containing newlines, and a new requirement makes the rule general for
  the panel's single-line rows - a row that declares one line renders one line
  whatever its text contains. The other affected surfaces have no single-line
  requirement to change; their fixes follow the intent their own code comments
  state.

## Impact

- `knot`: a new text helper beside `shorten_path` in `app_support.rs`;
  `panel_view/tool_call.rs` (the collapsed title), `workspace_window/panel/input.rs`
  (queued prompt row), `workspace_window/chrome.rs` (`detail_line`),
  `workspace_window/render/title_bar.rs`, `dashboard.rs`, and
  `workspace_window/panel/pane.rs`.
- No change to `knot-core`, to any stored shape, or to what any agent-facing
  surface accepts: the flattening is presentational and the underlying strings
  are untouched.
- No new dependency, and no behavior change for any text that already had no
  newlines - which is why the defect went unnoticed until a heredoc appeared in
  a command.
