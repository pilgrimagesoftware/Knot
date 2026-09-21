# Tasks

## 1. Persist session setup and display preference

- [x] 1.1 Extend durable agent/session settings with optional model, permission mode, and effort fields; add decode defaults and verify legacy records load in focused persistence tests.
- [x] 1.2 Add the compact tool-call display preference to the single settings store with a disabled default; verify it round-trips and legacy settings decode successfully.
- [x] 1.3 Wire model, permission, and effort selectors to save changes and apply them only to the next turn; verify an in-flight turn keeps its original setup.

## 2. Add compact tool-call aggregation

- [ ] 2.1 Add transient panel state that groups contiguous tool events and closes groups at prompt or non-tool output boundaries; verify interleaved event sequences produce separate summaries.
- [ ] 2.2 Derive call, file, and command counts from structured tool metadata with graceful omission when metadata is unavailable; verify counts update as results arrive.
- [ ] 2.3 Render the live summary line with failure indication and an inspection path to underlying tool-call records; verify no tool result is discarded.
- [ ] 2.4 Keep existing individual tool-card rendering as the default and switch between modes without changing stored event state; verify both modes render the same underlying calls.

## 3. Settings UI and verification

- [ ] 3.1 Add the compact tool-call mode control to the settings/panel UI with localized labels and an off default; verify the preference is immediately persisted.
- [ ] 3.2 Add focused tests for session setup restoration, summary boundaries, count updates, failures, and mode switching; verify the relevant Rust test target passes.
- [ ] 3.3 Run formatting, lint, and the full test suite; verify `cargo +nightly fmt --check`, project lint, and `cargo test` pass.
