# Tasks

## 1. Make room in the composer

- [ ] 1.1 Split `workspace_window/panel/input.rs` (517 lines, cap 700) by concern — chip strip, queued-prompt list, control bar, text entry — into sibling files, leaving `mod.rs` declaring only; verify `make size-check` and `make lint` pass and `make test` is unchanged
- [ ] 1.2 Add a panel-composer test module covering today's behaviour before anything moves: send key honours `agent_panel_shift_enter_sends`, the other chord inserts a newline, expand/collapse switches between `PANEL_INPUT_ROWS_COLLAPSED` and `PANEL_INPUT_ROWS_EXPANDED`, and a selected Panel agent takes focus; verify all pass against the current `TextareaState`

## 2. Unblock styling plus auto-grow

- [ ] 2.1 Fork `gpui-base`, moving `auto_grow`, `set_auto_grow`, `rows` and `set_rows` from `impl InputBaseState<TextareaMode>` to `impl<M: MultiLineMode> InputBaseState<M>` with no body change; verify the fork builds and its own test suite passes
- [ ] 2.2 Open the upstream PR at `longbridge/gpui-kit` with that diff and a note that `element.rs` already drives `AutoGrow` generically over the mode marker; verify the PR URL is recorded in the ADR from 2.4
- [ ] 2.3 Wire `[patch.crates-io]` for `gpui-base` in the root `Cargo.toml`; verify `make` passes end to end and `cargo tree` shows one `gpui-base`, the patched one
- [ ] 2.4 Write `docs/adr/` recording the fork, why the bound blocks the composer, the upstream PR link, and the exit condition (delete the patch when a release carries the relaxed bound); verify it is indexed in `docs/adr/README.md`

## 3. Swap the widget, preserving behaviour

- [ ] 3.1 Change `panel_prompt_input` to build `EditorState::new(..).placeholder(..).submit_on_enter(..).auto_grow(1, max_rows)` and `set_panel_input_expanded` to call `set_auto_grow` on it; verify every test from 1.2 passes unchanged against `EditorState`
- [ ] 3.2 Move the render site from `Textarea` to `Editor`, keeping the drag-over, `on_drop`, `capture_action::<Paste>` and lookup wiring intact; verify dropping a file, pasting a screenshot and pasting text each behave as before
- [ ] 3.3 Add tests for `panel-rich-input`'s "The composer stays a composer": no line numbers, gutter, indent guides or fold controls are drawn; typing `(` leaves `(` alone; soft wrap is on; the input grows to the same bound as before
- [ ] 3.4 Verify by hand in a debug build that IME composition, the context menu, selection by mouse and keyboard, and undo/redo behave as they did before the swap; record the walkthrough in the change

## 4. The scanner

- [ ] 4.1 Add a composer scanner module — a pure function over `&str` plus the attachment table, returning ranges by construct family, with no GPUI types in its signature; verify unit tests run without a window
- [ ] 4.2 Implement token recognition for `/` and `@`, including the word-boundary rule; verify `crates/knot/src` and `paul@example.com` yield no token, while a leading `/` and an `@` after a space do, and an unresolvable `/nosuchcommand` still scans as a token
- [ ] 4.3 Implement markdown recognition — emphasis, strong, inline code, fenced blocks, headings, list markers, block quotes, links — with unclosed constructs falling back to prose; verify `a * b` yields no emphasis and an unclosed fence styles only the lines after it
- [ ] 4.4 Give fenced blocks one code treatment regardless of info string; verify a `rust` block and a `python` block scan identically
- [ ] 4.5 Implement dirty-region rescanning: an edit's line widened to the enclosing block construct, widened again while a fence can have opened or closed; verify against a whole-buffer rescan over the edit sequences from `panel-rich-input`'s "survives every way the composer changes" — the two must agree
- [ ] 4.6 Add a scaling test asserting the work a single-character edit does is bounded by the edited construct, not the buffer length; verify it fails if the scanner is changed to rescan everything

## 5. Painting the styling

- [ ] 5.1 Build the three decoration collections — attachment chips, tokens, markdown, created in that order for precedence — from `cx.theme()` at build time; verify each treatment is visually distinct in a debug build
- [ ] 5.2 Drive a rebuild from `InputEvent::Change` through the dirty-region scanner; verify typing, pasting, undo, redo, cut and drag-of-text each leave every range styled as though the text had been typed
- [ ] 5.3 Style the composer on arrival of text it did not receive by keystroke — a restored draft and a lookup insertion; verify a restored markdown draft is styled with no edit
- [ ] 5.4 Verify no treatment relies on hue alone: each also carries weight, slant or a background, and every one stays legible in both system appearances; check a light/dark switch restyles with no edit
- [ ] 5.5 Verify the buffer is untouched by styling — send a prompt containing markdown and assert the agent receives it byte-identical, and assert copying a styled token yields its raw characters

## 6. `@` file mentions

- [ ] 6.1 Make `active_token` in `panel_commands/token.rs` trigger-aware, returning which trigger is under the caret; verify a buffer holding both a `/` and an `@` token reports the one the caret sits in
- [ ] 6.2 Move matching and ranking from `LookupEntry::matches` onto `LookupSource`, leaving commands on case-insensitive substring; verify the existing `panel-slash-commands` tests pass unchanged
- [ ] 6.3 Add a file `LookupSource` with subsequence matching, a score preferring name matches over directory matches and consecutive characters over scattered ones, and matched-character indices for the row; verify `kgs` reaches `crates/knot-git/src/lib.rs` and ranking and marking match the spec's scenarios
- [ ] 6.4 Enumerate the agent's own working folder off the render path — via `knot-git` for a repository (tracked plus untracked-but-not-ignored), a filesystem walk with the Swift reference's exclusion set otherwise; verify an ignored `target/` path never appears and a new untracked file does
- [ ] 6.5 Cache the listing per agent and refresh it from `knot-discovery`'s debounced watch rather than a timer; verify a file created while the agent is open appears in a later lookup without reopening the agent
- [ ] 6.6 Report enumeration states in the popup — still gathering, and folder over the cap with matching still offered over what was gathered; verify the composer keeps accepting keystrokes during enumeration and add the localization keys
- [ ] 6.7 Dispatch the popup on the caret's trigger so the two lookups are mutually exclusive, sharing navigation, insertion and Esc; verify moving the caret between a `/` token and an `@` token switches the list and never shows both
- [ ] 6.8 Insert a mention as `@` plus the path relative to the agent's folder, keeping a path with a space one token, replacing only the typed token; verify `look at @knotg` becomes `look at @crates/knot-git/src/lib.rs` and that inserting an image's path attaches nothing and reads no file

## 7. Attachment chips

- [ ] 7.1 Insert a styled reference token at the caret when context arrives by add-context control, Finder drop or pasted image, alongside its existing `pending_context` row; verify all three paths produce both presences, and that attaching into an empty buffer leaves the caret after the token
- [ ] 7.2 Reconcile buffer to table after every edit, buffer-decides: a deleted token detaches its row; verify deleting a reference drops it from the strip and from what is sent
- [ ] 7.3 Remove a token as a programmatic edit when its strip entry is dismissed, flowing through the same reconciliation; verify removing the strip entry clears the reference from the buffer
- [ ] 7.4 Style attachment references as chips, distinct from prose and from a typed `@` mention; verify against `panel-rich-input`'s chip scenarios in a debug build

## 8. Close out

- [ ] 8.1 Run the full gate — `make` — and verify `fmt-check`, `size-check`, `lint`, `test` and `build` all pass, with no `.rs` file over 700 lines
- [ ] 8.2 Walk a debug build through every scenario in `panel-rich-input`, `panel-file-mentions` and the `acp-panel-ui` and `panel-slash-commands` deltas; record the walkthrough in the change
- [ ] 8.3 Resolve the two open questions from `design.md` — mention quoting versus escaping for paths with spaces, and base name versus relative path on an attachment chip — now that the styling is visible; verify the choice is reflected in the code and the walkthrough
- [ ] 8.4 Check whether the upstream PR from 2.2 has released; if so remove the `[patch.crates-io]` entry and the fork and verify `make` still passes, otherwise verify the ADR records the current upstream status
