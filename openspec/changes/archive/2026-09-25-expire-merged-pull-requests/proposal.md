# Proposal

## Why

A workspace's Pull Requests list only grows. Every pull request an agent has
ever opened stays in it, so merged work the user has already dealt with crowds
out the pull requests still needing attention, and the launcher row's counts
become a lifetime tally rather than a picture of what is outstanding. Today the
only way to clear a merged pull request is to remove each row by hand.

## What Changes

- A recorded pull request that has been merged for longer than a retention
  window (24 hours) is dropped from the workspace's list automatically, the
  same way a user-removed row is dropped: the record goes, nothing on GitHub
  changes.
- "Merged for longer than" is measured against the forge's own merge time, not
  against when Knot first saw the URL. `first_seen` records when the URL
  appeared in an agent's output, which says nothing about when the pull request
  merged.
- The forge query gains the merge timestamp it does not currently ask for, and
  the fetched state carries it, so the expiry rule has something to measure.
- Expiry is driven by fetched state. A record whose state has not been fetched,
  or whose fetch failed, is never dropped - Knot only removes what it has
  positively observed as merged and old.
- Closed and open pull requests are unaffected. So is the existing manual
  removal, which stays available for every state.

Non-goals, stated so the scope is not read wider than it is:

- Closed (unmerged) pull requests do not expire. The user asked for merged; a
  closed pull request is often abandoned work someone still wants to see.
- The retention window is not user-configurable in this change. It is a named
  constant, so making it a setting later is a small follow-on.
- No archive, undo or "recently expired" view. An expired record behaves
  exactly like a manually removed one: it is recorded again if an agent's
  output carries its URL again.
- Fetched pull request state remains unpersisted. This change does not
  introduce a cache on disk.

## Capabilities

### New Capabilities

None. This extends behavior the `pull-request-tracking` capability already
owns.

### Modified Capabilities

- `pull-request-tracking`: adds a requirement that a merged pull request is
  dropped from the list once it has been merged longer than the retention
  window; extends the "state is fetched and refreshed" requirement so the
  fetched state includes the merge time; and narrows the persistence
  requirement, which currently says a record survives until its agent or
  workspace is removed or the user removes it.

## Impact

- `crates/knot-forge`: `PULL_REQUEST_FIELDS` gains `mergedAt`;
  `RawPullRequest` and `PullRequestState` gain a merge timestamp; the
  `merged-*` testdata files gain the field.
- `crates/knot-agents`: `AgentStore` gains an age-based prune alongside
  `remove_pull_request` and the `forget_*` cascades.
- `crates/knot`: the Pull Requests view's refresh path runs the prune after a
  fetch resolves, reusing the existing store-mutate -> persist ->
  prune-state-cache sequence; `consts.rs` gains the retention window.
- `crates/knot-core`: no change to `SavedPullRequest`. The merge time is read
  from fetched state, so nothing new is persisted and `pull-requests.json`
  keeps its current shape.
- No dependency changes. No migration: the persisted document is unchanged.
