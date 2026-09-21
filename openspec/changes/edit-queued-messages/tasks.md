# Tasks

## 1. Queue to composer

- [ ] 1.1 Add a take-by-stable-id operation to pending-message state that removes an editable entry and returns its text; verify the remaining queue keeps its order and an in-flight or absent id is refused.
- [ ] 1.2 Place the returned text in the panel composer and focus it; verify the row disappears and the composer holds the message's text.
- [ ] 1.3 Confirm before replacing composer text the user has typed; verify declining leaves both the queue entry and the composer unchanged, and that an empty composer is filled without a prompt.
- [ ] 1.4 Verify sending edited text enqueues it at the back of the queue and that editing during an active response leaves the streaming turn untouched.

## 2. Row control

- [ ] 2.1 Add an icon edit control to each queued-message row with a localized tooltip and accessibility label; verify it is absent for an in-flight entry and present for queued and failed ones.

## 3. Verification

- [ ] 3.1 Add tests for editing among duplicate message text, the composer-load path, the decline path, and the tooltip/accessibility labels; verify the relevant test target passes.
- [ ] 3.2 Run formatting, lint, and the full test suite; verify `cargo +nightly fmt --check`, project lint, and `cargo test` pass.
