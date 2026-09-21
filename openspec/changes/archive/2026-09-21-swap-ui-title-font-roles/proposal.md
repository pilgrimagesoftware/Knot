# Proposal

## Why

The two configurable proportional fonts are labelled backwards. The Appearance
tab's "UI" row edits `ui_font_name` (Manrope), which is applied to almost
nothing - the workspace header, the sidebar's secondary cell text, the About
window's credit lines, the panel input - while the "Title" row edits
`title_font_name` (Adamina), which is the app-wide default that draws
essentially the whole interface. A user who wants to change the font the app is
written in has to find it under "Title", and the field names mislead every
future reader of the code the same way.

The Markdown renderer sits on the other side of the same gap: it inherits the
app-wide family for everything, so a header is drawn in the body face at a
larger size rather than in a display face of its own.

## What Changes

- Swap the roles of the two proportional font settings so each name describes
  what it draws: `ui_font_name` / `ui_font_size` become the app-wide body and
  interface font (default Adamina, 16pt), and `title_font_name` /
  `title_font_size` become the title and header font (default Manrope, 14pt).
- Swap every consumer with them, so the app renders exactly as it does today:
  the theme's default family and size now read `ui_font_*`, and the sites that
  apply a family explicitly (workspace header, sidebar secondary text, About
  credits, panel input) now read `title_font_*`.
- **BREAKING** (persisted shape): migrate an existing `settings.json` once on
  load, swapping the values of `uiFontName`/`titleFontName` and
  `uiFontSize`/`titleFontSize` so a user's customized fonts keep the text they
  were chosen for. A new `settingsVersion` scalar records that the migration
  has run; a document without it is treated as pre-migration. Only keys present
  in the document are swapped, so a document that never customized fonts keeps
  the new defaults rather than inverting them.
- Render Markdown body text - paragraphs, lists, tables, quotes, inline code's
  surrounding text, everything the renderer draws - in the UI font (Adamina),
  and render `#`-`######` headers in the title font (Manrope), at the heading
  level's existing size and weight. Applies to both Markdown surfaces: the ACP
  panel's assistant messages and the Markdown pane the `display-markdown` MCP
  tool opens.
- Inline formatting inside a header (bold, italics, links, inline code) is
  flattened to plain text in the title font. gpui-kit exposes no per-heading
  font family, so a header is rendered by a Knot-side block renderer that
  receives the header's text, not its parsed inline marks.

### Non-goals

- Re-pointing which chrome text uses which family. Manrope stays on the
  workspace header, the sidebar's secondary cell text, the About credits and
  the panel input; Adamina stays everywhere else. This change renames and
  re-wires, it does not restyle - the rendered app is pixel-identical apart
  from Markdown headers.
- Renaming the Appearance tab's row labels. The rows stay "UI", "Title" and
  "Terminal"; what changes is which text each one governs.
- Any new font setting, and any change to the terminal font.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `settings-persistence`: the two proportional font scalars change meaning -
  `ui_font_*` is the app-wide interface font, `title_font_*` the title font -
  with stated defaults, plus a new requirement for the one-time migration of a
  pre-migration document and the `settingsVersion` marker that gates it.
- `settings-ui`: the Appearance tab requirement states which text each Fonts
  row governs, so the binding between a row and the interface is part of the
  contract rather than an accident of which field it happens to write.
- `acp-panel-ui`: a new requirement for the Markdown renderer's two faces -
  body text in the UI font, headers in the title font - including the flattening
  of inline marks inside a header.

## Impact

- `knot-core`: `consts.rs` (the four font default constants swap values),
  `settings/store.rs` (`settingsVersion` field, load-time font-role migration,
  tests).
- `knot`: `app_support.rs` (theme family and size read `ui_font_*`; the
  embedded-font doc comment), `app_bootstrap.rs` and `about_window/mod.rs` (the
  About window's credit-line family), `workspace_window/render/*` (header and
  sidebar family and size), `workspace_window/panel/input.rs`,
  `workspace_window/panel/pane.rs` and `panel_view/mod.rs` (both Markdown
  surfaces), plus a new Markdown-styling module holding the heading block
  parser and renderer.
- No new dependency: the heading hooks (`markdown_block_parser`,
  `markdown_block_renderer`) and the `markdown_ast` AST types are already
  re-exported by `gpui-kit` 0.6.
- A user's existing `settings.json` is rewritten once on the next persist,
  gaining `settingsVersion` and the swapped font values.
