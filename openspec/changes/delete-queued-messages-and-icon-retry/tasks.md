# Tasks

## 1. Queue deletion

- [ ] 1.1 Add stable-id removal to pending-message state and verify deleting one entry preserves the order of the remaining queue.
- [ ] 1.2 Add an icon delete control to each queued-message row with localized tooltip and accessibility label; verify it removes only the selected row.
- [ ] 1.3 Verify deletion during an active response leaves the active turn and other queued messages unchanged with focused panel tests.

## 2. Retry control

- [ ] 2.1 Replace the retry text button with the existing icon-button primitive while retaining its callback; verify retry still submits the same message.
- [ ] 2.2 Add localized retry tooltip and accessibility label and verify keyboard focus exposes the same action name.

## 3. Verification

- [ ] 3.1 Add or update UI tests for queued deletion, duplicate message text, retry activation, and tooltip/accessibility labels; verify the relevant test target passes.
- [ ] 3.2 Run formatting, lint, and the full test suite; verify `cargo +nightly fmt --check`, project lint, and `cargo test` pass.
