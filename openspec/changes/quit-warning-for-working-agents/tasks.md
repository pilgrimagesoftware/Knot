# Tasks

## 1. Quit State Guard

- [x] 1.1 Trace all application quit entry points and route them through one guarded quit path, verifying menu, keyboard, window-close, and system termination requests use the same decision
- [x] 1.2 Add a working-agent count derived from the existing agent state store, verifying idle, stopped, and completed agents do not trigger the guard

## 2. Confirmation Dialog

- [x] 2.1 Add localized warning title, message, Quit action, and Cancel action for one and multiple working agents, verifying the rendered copy and accessibility labels
- [x] 2.2 Add a one-shot confirmed-quit bypass that invokes normal shutdown exactly once, verifying confirmation does not reopen the warning

## 3. Verification

- [x] 3.1 Add tests for idle quit, one working agent, multiple working agents, cancel, confirm, and state changes while the dialog is open, verifying all quit-warning scenarios
- [x] 3.2 Run nightly formatting, clippy with warnings denied, and the workspace test suite, verifying all checks pass
