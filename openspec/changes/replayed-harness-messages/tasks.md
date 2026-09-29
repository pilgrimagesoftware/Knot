# Tasks

## 1. Metadata on user chunks

- [x] 1.1 Keep `_meta` on `SessionUpdate::UserMessageChunk` in `knot-acp`;
      verify with `protocol` tests for a chunk with and without it.

## 2. Classifying replayed chunks

- [x] 2.1 Add `panel_state::harness::classify_user_chunk`, deciding by
      `_claude/origin` when present and otherwise by a chunk made entirely of
      harness blocks; verify with `harness/tests.rs` for both paths, a prompt
      that mentions a tag, and malformed blocks.
- [x] 2.2 Fold an injected chunk into a `PanelMessage::Notice` or nothing,
      never a user message; verify with `tests/replay.rs`.
- [x] 2.3 Render `Notice` as a compact muted single-line row, with text from
      `panel.harness.*`; verify the keys resolve in `tests/l10n_catalog.rs`.

## 3. Verification

- [ ] 3.1 Verify in the running app: resume a Claude agent whose conversation
      holds a background-task notification, and see a notice row where the
      prompt bubble was.
