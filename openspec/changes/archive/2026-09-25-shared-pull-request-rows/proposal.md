# Proposal

## Why

A pull request that several agents in one workspace worked on is listed once
per agent, and counted once per agent (#444). In a live document one pull
request is recorded against six agents: the pane draws six rows for it and the
launcher row counts six open pull requests where there is one.

Recording is right. `pull-request-tracking` records the same pull request once
per agent on purpose, because who opened it is part of what is recorded. The
defect is in the reading:

- `pull_request_groups` buckets records by agent, so a shared pull request lands
  in every owning agent's group.
- `workspace_pull_request_urls` returns one URL per record rather than per pull
  request, so `counts_for` counts it once per agent, and the refresh loop walks
  the duplicates.

## What Changes

- Records are grouped by URL before they are grouped by agent. A URL one agent
  recorded goes in that agent's group, as today. A URL several agents recorded
  is one row, in a group headed by all of them; URLs with the same set of
  agents share that group.
- Shared groups are drawn before single-agent ones. Rows stay newest first
  within every group, a shared row taking the earliest time any agent first saw
  it.
- `workspace_pull_request_urls` yields each URL once, which fixes the launcher
  row's count and the refresh loop.
- Removing a shared row removes the record for every agent it is attributed to:
  one row on screen, one removal.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-tracking`: "The workspace window lists its pull requests" lists
  and counts a pull request once however many agents recorded it, and gains a
  shared group. A new requirement covers removing a shared row.

## Impact

- `crates/knot/src/pull_request_groups.rs`: new, the pure grouping over records.
- `crates/knot/src/workspace_window/pull_requests_view.rs`: URLs de-duplicated;
  groups built from the pure grouping; removal addresses every owning agent.
- `crates/knot/src/workspace_window/render/pull_requests_pane.rs`:
  `PullRequestGroup` carries the owning agents; `render_row` takes them.
- No change to the store or the saved document. Attribution stays per agent.

## Non-Goals

- Changing how records are stored or identified.
- Removing a single agent's attribution from a shared row. The row a user
  removed reappearing under another heading reads as a bug.
