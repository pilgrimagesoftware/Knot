# Tasks

## 1. Localization

- [ ] 1.1 Add `panel.copy_code` ("Copy code") and `panel.copied_code` ("Copied
      code to clipboard") to `crates/knot-core/locales/en.yml`, beside the
      existing `copy_response` / `copied_response` pair; verify by adding a
      test in `crates/knot-core/src/l10n.rs` asserting both keys resolve to
      something other than the key string, matching `context_usage_key_resolves`
- [ ] 1.2 `touch crates/knot-core/src/lib.rs` before running the l10n tests, so
      the locale file is re-embedded rather than read from a stale build
      artifact; verify `cargo test -p knot-core` passes

## 2. The copy control

- [ ] 2.1 Add a pure `pub(crate) fn code_block_copy_text(block: &CodeBlock) ->
      SharedString` to `crates/knot/src/markdown_view.rs` returning
      `block.code()`, with a doc comment saying why the parser's own text is
      used rather than re-scanning the source (design.md - "The copied text is
      `CodeBlock::code()`"); verify it compiles under `cargo build -p knot`
- [ ] 2.2 Add a private `code_block_actions` renderer to `markdown_view.rs`
      building a ghost, small `Button` with `IconName::Copy`, the
      `panel.copy_code` tooltip, and an `on_click` that writes
      `code_block_copy_text(block)` to the clipboard via
      `cx.write_to_clipboard(ClipboardItem::new_string(..))` and pushes
      `Notification::info(t("panel.copied_code"))`; mirror the handler in
      `crates/knot/src/panel_view/message.rs:170-180`. Verify `cargo build -p
      knot` succeeds
- [ ] 2.3 Wire it into the `markdown_view` constructor with
      `.code_block_actions(..)`, and extend that function's doc comment to say
      the control is upstream-placed (top-right, inside the block) so a future
      reader does not go looking for layout code here; verify `make lint`
      passes with no new warnings
- [ ] 2.4 Confirm the new imports are explicit per the crate's convention
      (`gpui_kit::assets::IconName`, `gpui_kit::component::button::{Button,
      ButtonVariants}`, `gpui_kit::component::notification::Notification`,
      `gpui_kit::component::{Sizable, WindowExt}`,
      `gpui_kit::base::text::CodeBlock`, `gpui_kit::{ClickEvent,
      ClipboardItem}`) with no glob added; verify by grepping the file for
      `::*` and finding none

## 3. Tests

- [ ] 3.1 In `crates/knot/src/tests/markdown_view.rs`, add tests for
      `code_block_copy_text` built on `CodeBlock::from_code`: a fenced block
      with a language tag copies only its lines, a block whose code contains
      backticks copies them verbatim, and an empty block copies an empty
      string. Verify `cargo test -p knot markdown_view` passes
- [ ] 3.2 Extend that test module's doc comment to record what stays untested
      and why — the renderer itself needs a `Window` and an `App`, so the
      button's placement and its click handler are covered by the spec and by
      manual check, not by a unit test

## 4. Verification

- [ ] 4.1 Run `make` and confirm the full gate passes: `fmt-check`,
      `size-check` (`markdown_view.rs` must stay under 700 lines), `lint`,
      `test`, `build`
- [ ] 4.2 Run the app, send a prompt whose answer contains a fenced code block,
      and confirm against the spec's scenarios: the button sits in the block's
      upper-right corner without hovering, clicking it puts the block's code
      alone on the clipboard, the confirmation notification appears, a second
      block copies its own content, and an inline code span has no button
- [ ] 4.3 Open a Markdown file for an agent with `display-markdown` and confirm
      the same control appears on its code blocks, per the spec's "The Markdown
      pane carries the same control" scenario
- [ ] 4.4 Confirm "copy response" on the same message still copies the whole
      response, code block included, per the spec's last scenario
