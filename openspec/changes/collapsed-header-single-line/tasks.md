# Tasks

## 1. The helper

- [ ] 1.1 Add `single_line(text: &str) -> String` to `crates/knot/src/app_support.rs` beside `shorten_path`, collapsing every run of whitespace to one space and trimming, with a doc comment recording why the style flags are not enough - `gpui::text_system::shape_text` splits on `\n` before any wrapping or truncation decision, so `whitespace_nowrap` cannot make multi-line text one line; verify `make lint` is clean.
- [ ] 1.2 Add unit tests for it: a single-line string passes through unchanged, a string with `\n` joins with a single space, indentation and tabs collapse rather than surviving, runs of spaces collapse, leading and trailing whitespace go, an empty string and a whitespace-only string both come back empty; verify `make test` runs them.

## 2. The reported defect

- [ ] 2.1 Flatten the title in `render_tool_call_card`'s `collapsed` branch in `crates/knot/src/panel_view/tool_call.rs`, leaving the expanded branch to render `label` verbatim; verify in the running app that a collapsed call whose title is a heredoc command is one line ending in an ellipsis, and that expanding it shows the command's own line breaks.
- [ ] 2.2 Verify the rest of the collapsed header is unaffected: the disclosure chevron, the tool icon and the status indicator stay on that one line, and a short multi-line title that fits once joined shows in full with no ellipsis.

## 3. The same defect elsewhere

- [ ] 3.1 Flatten the queued prompt's text in the row rendered by `crates/knot/src/workspace_window/panel/input.rs`, leaving `prompt.text` itself untouched; verify a queued multi-line prompt occupies one row, and that delivering it sends the prompt as written and shows it in the conversation across as many lines as it needs.
- [ ] 3.2 Flatten the text once in `detail_line` (`crates/knot/src/workspace_window/chrome.rs`), before its size branch; verify with an agent whose status text carries a newline - set through the `set-status` MCP tool - that the sidebar row stays the height it is at, and that a long persona name still soft-wraps as its comment describes.
- [ ] 3.3 Flatten the activity text and the folder in `crates/knot/src/workspace_window/render/title_bar.rs`; verify the same multi-line status leaves the workspace title bar one line tall.
- [ ] 3.4 Flatten the activity text and the folder on the dashboard card in `crates/knot/src/dashboard.rs`; verify the card keeps its shape with that same status set.
- [ ] 3.5 Flatten the Markdown pane's title in `crates/knot/src/workspace_window/panel/pane.rs`; verify the pane header stays one line for a path containing a newline.
- [ ] 3.6 Grep for `whitespace_nowrap` across `crates/knot/src` and confirm every remaining site either passes through the helper or renders text Knot formats itself - `diff_stats_row` is the only expected exception; note it in that function's comment if it is not already clear.

## 4. Verification

- [ ] 4.1 Walk the changed and added scenarios in `specs/acp-panel-ui/spec.md` against the running app: the long-title ellipsis, the indicator on the line, an expanded card unchanged, the multi-line command collapsed, the expanded card keeping line breaks, the short joined title with no ellipsis, the queued multi-line prompt's single row, the delivered prompt, and the same prompt in the conversation.
- [ ] 4.2 Run `make` and `make size-check` and verify the whole workspace is clean.
