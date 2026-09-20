# Tasks

## 1. ACP Cancellation

- [x] 1.1 Trace the existing ACP turn lifecycle and expose a session-scoped cancellation operation, verifying the adapter compiles and existing completion behavior remains unchanged
- [x] 1.2 Make cancellation idempotent while a request is pending and normalize cancellation/completion races through the existing terminal-state path, verifying targeted ACP tests pass

## 2. Panel Controls

- [x] 2.1 Add the localized stop control beside the panel send control and show it only during an active cancellable turn, verifying the control state through panel UI tests
- [x] 2.2 Wire the control to the selected session and restore normal input after cancellation or natural completion, verifying another active panel session is unaffected

## 3. Verification

- [x] 3.1 Add or update tests for stop visibility, duplicate activation, session scoping, and terminal-state reset, verifying `cargo test --workspace` passes
- [x] 3.2 Run nightly formatting, clippy with warnings denied, and the workspace test suite, verifying all checks pass
