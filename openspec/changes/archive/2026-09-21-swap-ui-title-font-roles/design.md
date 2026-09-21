# Design

## Context

See `proposal.md` - Why. The state that shapes the approach:

- `Settings` is one JSON document, loaded through `Settings::load_at`, which
  already reads the raw `serde_json::Value` before deserializing and already
  carries one load-time upgrade (the `"SF Mono"` terminal font). There is no
  version marker of any kind in the document today.
- `apply_visual_identity` sets `theme.font_family` / `theme.font_size` from
  `title_font_*`. Four sites apply `ui_font_*` explicitly: the workspace title
  bar, the agent sidebar rows, the About window's credit lines (snapshotted at
  action registration), and the panel input.
- Both Markdown surfaces go through `gpui_kit::component::text::TextView`:
  `TextView::markdown` in `panel_view` (assistant messages) and in
  `workspace_window::panel::pane` (the pane `display-markdown` opens).
- gpui-kit 0.6.4 exposes no per-heading font family. `TextViewStyle` carries
  `heading_base_font_size` and a `heading_font_size(level, base)` hook, plus
  `StyleRefinement`s for code blocks and tables - nothing for a heading's face.
  Headings are rendered by `gpui-base` as a plain `div` with the level's size
  and weight, inheriting the ambient family.
- gpui-kit does expose parser and renderer hooks on `TextView`:
  `markdown_block_parser(Fn(&markdown_ast::Node, &MarkdownParseContext) ->
  Option<MarkdownNode>)` and `markdown_block_renderer(name, Fn(&MarkdownNode,
  &mut Window, &mut App) -> impl IntoElement)`. The block parser runs ahead of
  the built-in AST conversion for every block node, so it can claim headings.
  `markdown_ast` (the `markdown` crate's `mdast`) is re-exported, so no new
  dependency is needed.

## Goals / Non-Goals

**Goals:**

- A single load-time migration point that future settings migrations can reuse,
  rather than another special-cased value check beside the `"SF Mono"` upgrade.
- Markdown heading styling defined in one place in the `knot` crate, used by
  both Markdown surfaces, so the two cannot drift.
- Rendering unchanged outside Markdown headers, verifiable by reading the diff:
  every consumer swap is mechanical.

**Non-Goals:**

- Upstreaming a heading `StyleRefinement` to gpui-kit. Worth doing eventually -
  it would delete the block parser and renderer here - but it is an external
  crate and this change must not wait on it.
- A general settings migration framework. One version integer and one migration
  function, with room to add the next.

## Decisions

### A monotonic `settings_version` integer, not a boolean marker

`Settings` gains `settings_version: u32` (`settingsVersion` in the document).
`SETTINGS_VERSION_CURRENT` is `1`; version `0` means the font roles are the old
way round.

A boolean such as `fontRolesSwapped` would read clearly for exactly this one
migration and be dead weight for the next. An integer lets the next migration
add a step rather than a field.

The field needs two different "missing" answers, so it carries its own serde
default: `Settings` has a container-level `#[serde(default)]`, which fills a
missing field from `Default::default()` - and `Settings::default()` must carry
`SETTINGS_VERSION_CURRENT` so a fresh install is not migrated. A field-level
`#[serde(default = "...")]` returning `0` overrides that for the deserialize
path, so a document without the key is correctly read as pre-migration while a
fresh `Settings::default()` is current.

### Migrate on the raw JSON `Value`, before deserializing

`load_at` already holds the parsed `Value`. The migration swaps the
`uiFontName` / `titleFontName` and `uiFontSize` / `titleFontSize` entries in
that object - moving only entries that are present - then sets
`settingsVersion` and lets serde fill the rest.

Swapping after deserialization cannot distinguish "the document said Manrope"
from "the field was absent and defaulted to Manrope", and would invert the
defaults for every user who never touched the fonts. Presence is only knowable
on the raw document, which is why the migration lives there.

The migrated value is not persisted eagerly; the next persist writes it, the
same as the existing `"SF Mono"` upgrade. The migration is idempotent in effect
either way, since it is gated on the version the document carries.

### Swap the defaults wholesale, sizes included

`UI_FONT_DEFAULT` / `UI_FONT_SIZE_DEFAULT` become `"Adamina"` / `16.0`;
`TITLE_FONT_DEFAULT` / `TITLE_FONT_SIZE_DEFAULT` become `"Manrope"` / `14.0`.
The sizes have to move with the families: the theme's default text size is
`title_font_size` (16) today and becomes `ui_font_size`, and the explicitly
applied size is `ui_font_size` (14) today and becomes `title_font_size`.
Swapping names but not sizes would silently resize the whole interface.

### Headings via a block parser and renderer, not a plugin type

The heading hooks are installed as two closures through
`markdown_block_parser` / `markdown_block_renderer("knot-heading", ...)`
rather than as a `MarkdownPlugin` implementation. The plugin trait exists to
package a parser and renderer together for reuse across applications; here both
closures live in one Knot module and capture the two values they need (the
title family, and the heading base size). A trait impl would add a type and a
`setup` method and carry the same two fields.

The parser claims `markdown_ast::Node::Heading`, reads `depth`, flattens the
heading's inline children to a string, and returns a `MarkdownNode` named
`knot-heading` carrying `(level, text)`. The renderer draws a `div` with the
title family, the level's size and weight, and the same bottom padding
`gpui-base` gives a heading, so only the face differs from the built-in
rendering:

| level | size factor | weight    |
| ----- | ----------- | --------- |
| 1     | 2.0         | Bold      |
| 2     | 1.5         | Semibold  |
| 3     | 1.25        | Semibold  |
| 4     | 1.125       | Semibold  |
| 5     | 1.0         | Semibold  |
| 6     | 1.0         | Medium    |

The factors and weights are `gpui-base`'s own; they are duplicated here because
claiming the node means the built-in branch no longer runs. A `gpui-kit` upgrade
that changes them will make the two disagree - see Risks.

### The heading base size is the configured Markdown body size

The size factors multiply a base. `gpui-base` uses
`TextViewStyle::heading_base_font_size`, which no Knot code sets, so headings
are scaled from its `14px` default regardless of the user's
`markdown_font_size`. The heading renderer uses `markdown_font_size` as the base
instead, and the surfaces pass the same value into `TextViewStyle`, so the two
agree and a user who enlarges the Markdown body size gets proportional headers.

This is the one rendering change beyond the two faces, and only for a user who
changed `markdown_font_size` from its default of 14 - at the default the sizes
are identical to today's.

### Body font applied on the `TextView` element

`TextView` implements `Styled`, so `.font_family(ui_font)` on it sets the
ambient family for everything it draws, which every body construct inherits.
Headings do not inherit it: the heading renderer sets the family explicitly, and
a child's family wins over the ambient one. Code blocks and inline code keep the
monospace family, which `TextViewStyle` sets from the theme.

### One helper, both surfaces

A new module in the `knot` crate (`markdown_view`) exposes one constructor
taking the element id, the Markdown source, the two families and the body size,
and returning the configured `TextView`. `panel_view` and
`workspace_window::panel::pane` both call it. The spec requires the two surfaces
to render alike; one constructor is what makes that true by construction rather
than by review.

## Risks / Trade-offs

- [A gpui-kit upgrade changes heading sizes or weights, and Knot's duplicated
  table silently disagrees with the built-in rendering] → The table and its
  source are named in the module's doc comment, and the constants live in one
  place. A heading-face refinement upstream would remove the duplication
  entirely; until then this is a known upgrade check.
- [Claiming the heading node loses inline marks inside a header - bold, links,
  inline code render as plain text] → Accepted deliberately (see `proposal.md`).
  A header carrying a mark renders as the words, not the Markdown source, so the
  loss is styling rather than legibility. Rendering a nested `TextView` per
  header would preserve marks at the cost of nested element ids and uncertain
  selection behavior.
- [A header stops participating in text selection or the streaming fade the way
  a built-in heading does, because it is a custom block] → Verify selection
  across a header and streaming a response that contains one before merging; if
  either regresses, the fallback is to leave headers in the body face and reopen
  the upstream question.
- [A user's customized fonts end up inverted, the exact failure the migration
  exists to prevent, if the marker is written without the swap or the swap runs
  twice] → The version field is set in the same function that performs the swap,
  the swap runs only at version `0`, and the scenarios in
  `specs/settings-persistence/spec.md` cover both-customized,
  one-customized, neither-customized, already-migrated, and round-trip.
- [The rename touches many call sites, and one missed site renders the wrong
  face] → The compiler catches nothing here: both fields are `String`/`f64` and
  a missed site compiles. The task list enumerates every site found by grep, and
  the change is reviewed by grepping for `ui_font_` and `title_font_` afterwards.

## Migration Plan

Forward: a pre-migration document is migrated on the next load, in memory, and
recorded on the next persist.

Rollback: a build from before this change reads a document carrying
`settingsVersion` and ignores the unknown key, but reads the swapped font values
under the old meaning - so the two fonts appear exchanged. Rolling back
therefore means either accepting that one-time inversion or re-exchanging the
two values in `settings.json` by hand. Worth stating in the PR; not worth a
compatibility shim for a single-user desktop application.
