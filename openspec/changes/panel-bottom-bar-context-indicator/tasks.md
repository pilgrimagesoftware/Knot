# Tasks

## 1. ACP usage state

- [ ] 1.1 Decode `usage_update` notifications with used and total context tokens, with parser tests.
- [ ] 1.2 Store the latest valid usage in panel state, with state tests.

## 2. Radial indicator

- [ ] 2.1 Render a segmented radial indicator in the panel bottom bar when usage is available.
- [ ] 2.2 Show used and total tokens in a localized tooltip; clamp visual fill to the valid range.
- [ ] 2.3 Keep attachment chips independent of context-window usage.

## 3. Verification

- [ ] 3.1 `make rust` passes clean.
- [ ] 3.2 Confirm an ACP usage update changes the indicator and its tooltip in the app.
