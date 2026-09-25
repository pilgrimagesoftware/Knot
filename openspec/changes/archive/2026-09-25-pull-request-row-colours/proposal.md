# Proposal

## Why

Feedback on the Pull Requests view (#456):

- A merged or closed row still shows its checks result, which nobody will act
  on once the pull request is decided.
- An open row's colour says only "ready" (green) or "blocked" (amber). The
  user wants it to say why: mergeable, conflicting, behind its base, or checks
  still running.

"Behind" is not fetched today. `gh pr view` is asked for `mergeable`, which
reports conflicts only; `mergeStateStatus` is the field that reports `BEHIND`.

## What Changes

- `gh pr view` is also asked for `mergeStateStatus`.
- `Mergeability` splits `Blocked` into `Conflicting`, `Behind`,
  `ChecksRunning` and a remaining `Blocked` (draft, failing checks, required
  review).
- Open rows are green when mergeable, orange when conflicting or blocked,
  yellow when behind, and blue while checks run. Merged stays purple and
  closed stays red.
- Merged and closed rows leave the checks result out of their detail line.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-tracking`: "A recorded pull request's state is fetched and
  refreshed" limits the checks result to open pull requests and states what
  an open row's colour distinguishes.

## Impact

- `crates/knot-forge/src/pull_request.rs`, `consts.rs`: the new field and the
  split enum.
- `crates/knot/src/workspace_window/render/pull_requests_pane.rs`:
  `state_color` and `detail_line`.
- `crates/knot/src/consts.rs`: `COLOR_PULL_REQUEST_BLOCKED` becomes
  `COLOR_PULL_REQUEST_BEHIND`; orange and blue reuse the agent palette.
