# Tasks

## 1. ACP usage state

- [x] 1.1 Decode `usage_update` notifications with used and total context tokens, with parser tests.
- [x] 1.2 Store the latest valid usage in panel state, with state tests.

## 2. Radial indicator

- [x] 2.1 Render a radial indicator in the panel bottom bar when usage is available.
- [x] 2.2 Show grouped used and total token counts in a localized tooltip; clamp visual fill to the valid range.
- [x] 2.3 Keep attachment chips independent of context-window usage.

## 3. Verification

- [x] 3.1 Workspace tests, Clippy, and build pass clean.
- [x] 3.2 Confirm an ACP usage update reaches panel state and the radial render path.
