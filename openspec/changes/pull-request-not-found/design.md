# Design

## Context

See proposal.md - Why. The pieces involved: `knot_forge::pull_request_state_with`
runs `gh pr view`; `RefreshCache<String, Option<PullRequestState>>` in
`knot/src/pull_request_state.rs` holds the answers; the pane and the launcher
row read the cache.

## Decisions

### Classify at the `gh pr view` call, in `knot-forge`

The only place the forge's wording is known is the crate that talks to it.
`fetch` maps a `ForgeError::Command` whose output carries one of
`consts::NOT_FOUND_MARKERS` to `ForgeError::NotFound`, matched
case-insensitively like `UNAUTHENTICATED_MARKERS`. Every caller of
`pull_request_state_with` gets the classification, and the stub runner tests
it without `gh`.

`NotFound` is not `is_persistent()`: that predicate is about the view as a
whole (no `gh`, not signed in), and a missing pull request says nothing about
the others.

### A three-way lookup, not a nested `Option`

`PullRequestLookup { Known(PullRequestState), NotFound, Failed }` replaces
`Option<PullRequestState>` as the cache value. The cache's own absent key
stays "never asked". An enum names the three finished answers where a nested
`Option` would add a third meaning to `None`.

### Final answers are skipped, not held by the cache

`RefreshCache` gains `holds(key, predicate)`, a lock-and-test that does not
clone the value. `refresh_pull_request_states` skips a URL whose lookup
`is_final()` before claiming it. The rule stays with the pull-request cache,
where "final" means something; the generic cache stays generic and gains only
a read. Removing a record already forgets its entry through
`prune_pull_request_states`, so nothing else has to clear a final answer.

Rejected: a terminal flag inside `RefreshCache`. It would give `diff_stats` a
concept it has no use for.

### The row's status is a pane-local enum

The pane's icon and detail line took `Option<PullRequestStatus>`, where `None`
meant pending. They now take `RowStatus { Known(PullRequestStatus), NotFound,
Pending }`, derived once from the lookup. A failed lookup maps to `Pending`,
which is what it showed before. A not-found row earns no colour, like any row
without a state.

### Not found counts as known

`PullRequestCounts::nothing_known` counts `not_found` as known. A workspace
whose only records are missing pull requests should say "2 not found", not "2
recorded".
