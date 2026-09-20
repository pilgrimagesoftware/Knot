# Tasks

## 1. Locate and update panel rendering

- [ ] 1.1 Locate the ACP panel queued-message row renderer and identify the existing theme font and failure-color lookups; verify the affected path with the current panel implementation.
- [ ] 1.2 Render queued message content in the theme monospace font with single-line ellipsis truncation while keeping the status element visible; verify long queued text stays one line and truncates.
- [ ] 1.3 Render `failed` status in the theme failure color and proportional font without changing non-failed statuses; verify failed and non-failed rows through focused UI tests or snapshots.

## 2. Verification

- [ ] 2.1 Run the focused ACP panel tests and verify queued, failed, and unchanged message rendering cases pass.
- [ ] 2.2 Run the repository formatting and test checks required by the touched crate and verify no regressions.
