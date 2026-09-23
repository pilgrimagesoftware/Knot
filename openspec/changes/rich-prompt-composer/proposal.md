# Proposal

## Why

The panel composer is a flat `Textarea`: every character renders in one
colour, so a `/command`, an `@file` reference and a fenced code block are
indistinguishable from prose until the prompt is sent. The user is composing
structured text — slash tokens the lookup just inserted, file paths, markdown
— and gets no confirmation that the structure landed. The slash lookup already
proves the composer knows about tokens (`panel-slash-commands`); it simply
cannot show what it knows.

## What Changes

- The composer renders its buffer with styling rather than as flat text:
  slash tokens, `@` references and markdown constructs each get a distinct
  treatment while remaining ordinary editable characters.
- A new `@` trigger opens a file lookup over the agent's worktree, inserting a
  path reference the same way `/` inserts a command token. The `@` and `/`
  lookups share one popup and are mutually exclusive.
- Markdown the user types — emphasis, inline code, fenced blocks, headings,
  list markers — is styled in place. The buffer's text is unchanged; only its
  presentation differs, so what reaches the agent is byte-identical to what
  was typed.
- Attached context (paperclip, drag-and-drop, pasted screenshot) gains an
  in-buffer reference token styled as a chip, so an attachment has a position
  in the prompt rather than only an entry in the strip above it.
- **BREAKING (internal)**: the composer migrates from `Textarea`/
  `TextareaState` to `Editor`/`EditorState`. Styling in `gpui-kit` 0.6 is
  reachable only from `EditorMode`; `TextareaMode` exposes neither
  `InputHighlighter` nor `TextDecorationCollection`. No user-facing behaviour
  is removed — see `design.md` for the behaviours the migration must preserve.

### Non-goals

- Rendering true inline widgets (a thumbnail image drawn inside the text
  flow). `gpui-kit` 0.6 has no inlay or inline-element API, and folds are
  line-granular, so an attachment reads as a styled text chip and its
  thumbnail stays in the strip above the input.
- WYSIWYG editing: markdown is styled, never hidden or replaced. `**bold**`
  keeps both pairs of asterisks visible.
- Styling the terminal composer (`terminal-input`) or any other input in the
  application.
- Syntax highlighting of fenced code by language. A fenced block gets one
  code treatment regardless of its info string.

## Capabilities

### New Capabilities

- `panel-rich-input`: how the panel composer's buffer is rendered — the token,
  markdown and attachment-reference treatments, what the styling is forbidden
  to change about the text itself, and how it behaves while the lookup is open
  and while a turn is in flight.
- `panel-file-mentions`: the `@`-triggered file lookup over the agent's
  worktree — what it lists, how it filters and ranks, and what it inserts.

### Modified Capabilities

- `panel-slash-commands`: the slash lookup no longer owns the popup outright.
  It must yield to an active `@` token, and its inserted token must be one the
  rich-input styling can recognise.
- `acp-panel-ui`: attached context gains a second presence. It no longer
  lives only in the strip above the input; it also has a styled reference at
  a position in the prompt, and the two stay in step.

## Impact

- `crates/knot/src/workspace_window/panel/input.rs` (517 lines, near the
  700-line cap) — the widget swap and the decoration wiring; will need
  splitting.
- `crates/knot/src/workspace_window/panel/lookup.rs` — generalised from
  slash-only to trigger-driven, gaining the `@` source.
- `crates/knot/src/workspace_window/panel/prompt.rs` — attachment paths gain
  an in-buffer reference alongside the `pending_context` entry.
- `crates/knot/src/panel_commands/token.rs` — token recognition extends past
  `/`.
- New module for the composer's `InputHighlighter` implementation and its
  `HighlightStyleResolver`, themed from `cx.theme()`.
- `knot-core` localization: new keys for the `@` lookup's empty and loading
  states.
- No new external dependency: the markdown and token scanning is small enough
  to own. The application's existing markdown path (`markdown_view.rs`, over
  gpui-kit's `TextView` block parser) produces rendered *elements* from a
  finished document; the composer needs styled *ranges* over a half-typed
  buffer, incrementally. They are not the same job and do not share code.
- Render-path risk: the composer re-renders per keystroke, so scanning must be
  incremental over the edited range, never a full re-parse of the buffer.
