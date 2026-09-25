# Tasks

## 1. Shared classification and cache support

- [x] 1.1 Add `RowCategory` with `RowCategory::of(Option<&PullRequestLookup>)` in the new `pull_request_filter` module, and rebuild `pull_request_state::counts_for` on it; verify the existing `pull_request_state` tests pass unchanged
- [x] 1.2 Add `RefreshCache::mark_all_stale()` (clears request times, keeps values) and `ForgeStatus::mark_stale()`; verify with unit tests that a key claims again after it and its value is still returned by `get`

## 2. Filtering and sorting (pure)

- [x] 2.1 Move `PullRequestRow`/`PullRequestGroup` into `pull_request_filter` and add `first_seen` (via `pull_request_groups::first_seen_by_url`), filled in `pull_request_groups()`; verify `make build`, the existing pane tests and a `first_seen_by_url` test pass
- [x] 2.2 Implement search matching (title, `#number`, `owner/repo`, URL, agent names; case-insensitive, trimmed, stateless rows matched on URL parts); verify unit tests covering each spec scenario under "The user can search the Pull Requests view"
- [x] 2.3 Implement the status and agent filters and `category_counts`, including the reset of an agent filter that names no listed owner; verify unit tests covering the status and agent filter scenarios
- [x] 2.4 Implement `PullRequestSort` (with `Display` via l10n) and `attention_rank`, sorting within groups with newest-first and URL tiebreaks; verify unit tests for each sort scenario, including that group order is unchanged
- [x] 2.5 Implement `apply` composing search, filters and sort and dropping empty groups; verify a unit test that all three combine and that an empty result is distinguishable from an empty workspace

## 3. Window state and actions

- [ ] 3.1 Add `PullRequestViewState` to `WorkspaceWindow`, create the search `InputState` on first render with a `Change` subscription that notifies; verify manually that typing narrows the list per keystroke and the state survives leaving and returning to the view
- [x] 3.2 Replace `remove_pull_request` with `remove_pull_requests(rows)` over a pure `pull_request_groups::remove_rows` (one lock pass, one persist, one prune) and route the single-row path through it; verify the existing removal tests pass
- [x] 3.3 Add the bulk removal confirmations (merged, closed, not found, all) over the shown rows, with count and shown-only wording; verify a test that a filtered Remove all leaves hidden rows in the store
- [x] 3.4 Add Refresh now (`mark_all_stale`, `mark_stale`, the one-shot `refresh_final` set honoured by a pure `claim_refreshes`); verify a test that a not-found URL is claimed again after Refresh now and not on the following cycle
- [x] 3.5 Add Copy URLs (shown rows, shown order, newline-joined); verify a test on the produced string

## 4. Rendering

- [x] 4.1 Add `render/pull_requests_toolbar.rs`: search field, status toggles with counts, agent picker, sort picker and list actions menu with disabled states; verify `make size-check` and that the toolbar is absent for a workspace with no records
- [x] 4.2 Draw the filtered-empty message with its Clear control in the pane (clears search, statuses and agent, keeps sort); verify a pane test for the message choice between "none" and "no match"
- [x] 4.3 Add the row context menu (Open in browser, Copy URL, Remove) on the row in `render/pull_requests_pane.rs`, with a secondary click not opening the browser; verify with a probe in `tests/pull_request_row_clicks.rs`
- [x] 4.4 Add every new `pull_requests.*` key to `crates/knot-core/locales/en.yml` and assert in tests that each resolves; verify `make test`

## 5. Verification

- [x] 5.1 Run `make` (fmt-check, size-check, lint, test, build) and verify it passes
- [ ] 5.2 Manually verify in the app: search, each filter, each sort, Refresh now recovering a not-found row, Copy URLs, a filtered bulk removal and Remove all, the launcher row unaffected by filters, and the state reset after a relaunch
