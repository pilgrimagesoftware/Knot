# Proposal

## Why

A recorded pull request that the forge says does not exist sits at "Checking…"
forever (#441). The row never resolves, the launcher row counts it as pending
for as long as the window is open, and `gh` is re-run against it every refresh
cycle to produce an answer that is thrown away.

The error is discarded before anything can act on it.
`refresh_pull_request_states` records `pull_request_state_with(..).ok()`, so a
404, a timeout and a parse failure all become `None`; the row and the launcher
count then read `None` as pending. "The forge says this does not exist" and
"nothing has been asked yet" are the same row and the same number.

The forge does distinguish it. A nonexistent repository answers `Could not
resolve to a Repository with the name 'owner/repo'.` and a nonexistent number
`Could not resolve to a PullRequest with the number of N.`, both as a failed
command whose output Knot already carries.

## What Changes

- `knot-forge` classifies those two answers as a new `ForgeError::NotFound`,
  once, where `gh pr view` is run.
- The state cache holds a three-way lookup - known, not found, failed - instead
  of `Option<PullRequestState>`. An absent key still means "never asked".
- A not-found row gets its own icon and a `pull_requests.not_found` label. It
  stays clickable and removable.
- The launcher row gains a not-found count, shown only when it is not zero. A
  pull request that does not exist never counts as open or as pending.
- Not found is final for the window's lifetime, so it is no longer re-fetched.
  Fetched state is never persisted, so a relaunch asks again.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-tracking`: a new requirement for the not-found answer; "Knot
  degrades rather than fails" names it as not a failure to fetch; "The
  workspace window lists its pull requests" counts it apart.

## Impact

- `crates/knot-forge`: `error.rs`, `consts.rs`, `pull_request.rs` and its tests.
- `crates/knot`: `pull_request_state.rs`, `refresh_cache.rs`,
  `workspace_window/pull_requests_view.rs`,
  `workspace_window/render/pull_requests_pane.rs`,
  `workspace_window/render/pull_requests_row.rs`, their tests, and
  `knot-core/locales/en.yml`.

## Non-Goals

- Deleting a not-found record automatically. A private repository this `gh`
  identity cannot read gets the same answer as one that does not exist, so
  deletion would be guessing with the user's data.
- Distinguishing a failed fetch from a pending one on the row. A failed fetch is
  retried, so "Checking…" remains true of it.
