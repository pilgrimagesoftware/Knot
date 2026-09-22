# Design

## Context

See proposal.md - Why. The mechanics this rests on:

- The panel prompt is a `Entity<TextareaState>` held per agent id in the
  workspace window, with a `PressEnter` subscription per input - the input
  path is already hookable before a prompt is sent.
- The input area is a fixed stack (chips, prompt row, control row) rendered
  by `render_panel_input_area`; there is no menu or popover in the panel
  today, so the popup is a new in-panel element, not a new dependency.
- Knot already ships a vocabulary to draw on in the app itself:
  `plugin/claude/commands/*.md` defines knot's commands, and the selected
  agent's configured skill roots hold `SKILL.md` files (name + description in
  frontmatter). Nothing wires either into the panel today.

## Goals / Non-Goals

**Goals:**

- A `/` invites discovery: any entry is reachable by typing a prefix of its
  token or description, no settings page involved.
- The registry is text-only (token + description), so no crate below the UI
  window grows an execution contract it does not have.
- Insertion is reversible and predictable: it replaces the token and touches
  nothing else.

**Non-Goals:**

- No command execution: the panel completes text; the agent interprets it.
- No skill management (install, remove, update) - skills are read, not
  written.
- No argument forms or per-command schemas: an entry completes to its token,
  and its description says what it takes.
- No fuzzy ranking beyond a prefix/substring filter.

## Decisions

### The active token is a leading slash on the caret's line
A token is active when the first non-whitespace character of the line the
caret sits on is `/`, and the filter text is the characters between that `/`
and the end of the token (whitespace or the caret ends it). A `/` in the
middle of prose does not open the popup. This matches how the agents' own
slash vocabularies behave and keeps the popup out of ordinary typing.

### Registry v1: a built-in command list plus discovered skills
Two sources, one `(token, description)` entry type:
- **Built-in commands**: a documented, in-code list in a `panel_commands`
  module under `crates/knot`, seeded with the command tokens the app already
  names (its `plugin/claude/commands/*.md` set), token and one-line
  description per entry.
- **Skills**: scan the selected agent's configured skill roots (global and
  per-project, the roots `SKILL.md` lives in), read each skill's frontmatter
  name and description, and index them by token with their `/`-style name.
A root that is missing or unreadable yields no entries rather than an error,
per the spec. The two sources feed one registry interface so a third source
later is a provider, not a rewrite of the popup.

### The popup is an in-panel element anchored above the textarea
A self-contained listing rendered inside the input area directly above the
prompt row, so it cannot be clipped by the conversation scroll container and
does not cover the line being typed. Arrow-key handling is attached to the
same key-capture path that already owns `PressEnter`, so the lookup sees
keystrokes before they become text.

### Insertion replaces the token and puts the caret after it
The replacement spans the token's start to its end, writes the entry's full
token, and leaves the rest of the buffer alone. The caret lands just after
the inserted token. If the active token fills the whole buffer, the result is
simply the token.

### Escape and dismissal rules are shared
Esc, a matchless filter, an editing token gone, and blur all close the popup.
One dismissal pathway, so escaping out of a half-typed lookup can never leave
the popup open behind the terminal.

## Risks / Trade-offs

- [gpui-kit's `TextareaState` may not expose a reliable caret/selection for
  surgical token replacement] → The first task prototypes token detection and
  replacement against the real textarea API; if it cannot express a span
  replace, fall back to replacing the whole buffer when the token is all the
  text, or appending at the caret, and note the deviation. The spec only
  promises the observable result.
- [Scanning skill roots on every keystroke is wasteful] → v1 scans lazily per
  focused agent with a tiny memo; refresh on agent switch. A full watcher is
  explicitly out of scope.
- [The built-in command set is duplicated in the plugin command files] →
  Seeding from the names the plugin directory already defines keeps one
  vocabulary; the in-code list is the panel-side mirror, documented so a
  command added to the plugin dir is added there too.