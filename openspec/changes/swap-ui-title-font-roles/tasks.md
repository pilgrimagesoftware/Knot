# Tasks

## 1. Settings model and migration (`knot-core`)

- [x] 1.1 Swap the four font default constants in `crates/knot-core/src/consts.rs` - `UI_FONT_DEFAULT` to `"Adamina"`, `UI_FONT_SIZE_DEFAULT` to `16.0`, `TITLE_FONT_DEFAULT` to `"Manrope"`, `TITLE_FONT_SIZE_DEFAULT` to `14.0` - each with a doc comment naming the text it draws; verify `cargo test -p knot-core` still compiles and the existing settings tests pass.
- [x] 1.2 Add `SETTINGS_VERSION_CURRENT: u32 = 1` to `crates/knot-core/src/consts.rs`, documenting that version `0` means the pre-swap font roles; verify it is referenced from `settings/store.rs` in 1.3 rather than left unused (`make lint` reports dead constants).
- [x] 1.3 Add `settings_version: u32` to `Settings` with a field-level `#[serde(default)]` function returning `0` and `Settings::default()` carrying `SETTINGS_VERSION_CURRENT`; verify with a test that `Settings::default().settings_version == SETTINGS_VERSION_CURRENT` and that a document `{}` loads as version `0`.
- [x] 1.4 Add the raw-`Value` migration in `Settings::load_at`: at version `0`, exchange the `uiFontName`/`titleFontName` and `uiFontSize`/`titleFontSize` entries, moving only entries present in the document, then set `settingsVersion` to current; verify with tests for both-customized, one-customized-only, neither-customized, and already-migrated documents, per the scenarios in `specs/settings-persistence/spec.md`.
- [x] 1.5 Add a round-trip test in `crates/knot-core/tests/settings.rs`: load a pre-migration document, persist it, load again, and verify the font values are unchanged by the second load and the written document carries `settingsVersion`.
- [x] 1.6 Update the `settings/store.rs` module doc comment to name the font-role migration alongside the existing `"SF Mono"` upgrade; verify `make fmt-check` and `make test` pass.

## 2. Consumer swap (`knot`) - rendering must not change

- [x] 2.1 In `crates/knot/src/app_support.rs`, read `theme.font_family` and `theme.font_size` from `settings.ui_font_name` / `ui_font_size`, and rewrite the embedded-font doc comment and the inline comment above the theme assignment to describe the corrected roles; verify by running the app and confirming the interface is still drawn in Adamina at the same size.
- [x] 2.2 Swap the About window's credit-line family to `title_font_name` in `crates/knot/src/app_bootstrap.rs` and update the `register_about_action` doc comment in `crates/knot/src/about_window/mod.rs`; verify `crates/knot/src/tests/about_window.rs` passes with the field it constructs updated to `title_font_name`.
- [x] 2.3 In `crates/knot/src/workspace_window/render/mod.rs`, source the explicitly applied family and size from `title_font_name` / `title_font_size`, renaming the locals and the `agent_rows` / title-bar parameters in `render/sidebar.rs` and `render/title_bar.rs` to match, and update the comment in `render/mod.rs` that names which font is which; verify the workspace header and sidebar secondary text still render in Manrope at 14pt.
- [x] 2.4 Swap the panel input's family to `title_font_name` in `crates/knot/src/workspace_window/panel/input.rs`; verify the input's text renders unchanged.
- [x] 2.5 Grep `crates/` for `ui_font_` and `title_font_` and confirm every remaining site reads the field matching the text it draws; verify `make` passes.

## 3. Markdown heading renderer (`knot`)

- [x] 3.1 Add `crates/knot/src/markdown_view.rs` with the level-to-size-factor and level-to-weight tables from `gpui-base`, a doc comment naming that source and the upgrade check it implies, and a pure helper mapping a heading level plus a base size to its pixel size and font weight; verify with unit tests pinning all six levels and the out-of-range fallback.
- [x] 3.2 In the same module, add the block parser claiming `markdown_ast::Node::Heading` - reading `depth`, flattening the heading's inline children to plain text, returning a `MarkdownNode` named `knot-heading` carrying level and text; verify with a unit test that a heading containing a bold run flattens to the words without asterisks and that a non-heading node is not claimed.
- [x] 3.3 In the same module, add the block renderer for `knot-heading` - the title family, the level's size and weight, and the bottom padding `gpui-base` gives a heading - and one constructor taking element id, source, UI family, title family and body size that returns the `TextView` with the body family, the `TextViewStyle` heading base, and both hooks installed; verify it compiles and `make lint` is clean.
- [x] 3.4 Declare the module in `crates/knot/src/main.rs` and register its tests in `crates/knot/src/tests/mod.rs`; verify `make test` runs the new tests.

## 4. Markdown surfaces use the helper

- [x] 4.1 Route the panel's assistant messages in `crates/knot/src/panel_view/mod.rs` through the new constructor, adding the title family to `PanelStyle` beside `ui_font_family` and `mono_font_family` with a doc comment saying what it draws; verify a response with headers and paragraphs renders headers in Manrope and body in Adamina.
- [x] 4.2 Route the Markdown pane in `crates/knot/src/workspace_window/panel/pane.rs` through the same constructor with the same families and body size; verify a file shown through `display-markdown` renders the same two faces as the panel.
- [ ] 4.3 Verify the accepted losses and the risks named in `design.md` by hand: select text across a header, stream a response containing a header, and confirm inline code and fenced code blocks still render monospace.

## 5. Specs, docs and checks

- [ ] 5.1 Confirm the rendered result against `specs/acp-panel-ui/spec.md` scenario by scenario - header plus paragraph, list/table/quote, code, level sizes, a marked header, the Markdown pane, and changing the UI font with a response on screen.
- [ ] 5.2 Change the UI font and then the Title font from the Appearance tab and verify each row governs the text its label names, per `specs/settings-ui/spec.md`.
- [x] 5.3 Run `make` (fmt, clippy, test, build) and verify the whole workspace is clean.
