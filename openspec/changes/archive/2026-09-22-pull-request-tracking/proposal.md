# Proposal

## Why

An agent that opens a pull request prints its URL once, into a terminal that
keeps scrolling. Minutes later the URL is gone, and the only way back to it is
to remember which agent did the work and scroll its history, or to leave Knot
and search GitHub. With a workspace running several agents in parallel - which
is the whole point of Knot - the set of pull requests a session produced is the
session's actual output, and Knot currently does not keep it.

## What Changes

- Knot watches each agent's output for pull request URLs and records every one
  it sees against that agent: the URL, the agent and workspace that produced
  it, and when it was first seen. Both agent kinds are covered - the ACP panel's
  tool-call text for panel-mode agents, the terminal grid for terminal-mode ones.
- Recorded pull requests are persisted, so a session's output survives a restart
  of Knot rather than living only in a running window.
- Knot fetches each recorded pull request's state through the `gh` CLI - title,
  number, draft/open/merged/closed, and the check rollup - refreshing on a
  cadence and never on the render path.
- A Pull Requests view in the workspace window lists that workspace's recorded
  pull requests with their state, newest first, grouped by the agent that
  opened them. It is reached from a launcher row in the agent sidebar, shaped
  like the Dashboard row above it.
- Clicking a row opens the pull request in the user's default browser.
- A pull request can be removed from the list by the user. Knot never deletes
  anything on GitHub.
- When `gh` is absent or unauthenticated, recorded pull requests still list
  with their URLs and are still clickable; only the state column is absent, with
  the reason stated once rather than per row.

## Capabilities

### New Capabilities
- `pull-request-tracking`: what counts as a pull request Knot has seen, how one
  is detected from an agent's output, what is recorded about it, how its state
  is refreshed and what happens when it cannot be, and the view that lists them
  and opens one in a browser.

### Modified Capabilities
- `settings-persistence`: durable data gains a sixth collection document for
  recorded pull requests, alongside saved agents, workspaces, personas, bench
  templates and recent repositories.

## Impact

- New crate `knot-forge` (or a module under `knot-git`) wrapping the `gh` CLI:
  availability and auth probe, and `gh pr view <url> --json ...` for state.
  It runs `gh` through the same subprocess-with-timeout shape `knot-git`'s
  `Runner` uses and is runtime-agnostic for the same reason.
- `crates/knot-core/src/settings/` - a `SavedPullRequest` record, the new
  document in the store's document table, and its read/write path.
- `crates/knot-agents/src/store/` - the recorded-pull-request collection and
  its mutators, and per-agent/per-workspace lookups for the view.
- Detection taps: `crates/knot/src/panel_state/` for ACP `ToolCallContent::Text`,
  and `crates/knot-terminal`'s grid for terminal-mode agents.
- `crates/knot/src/open_in.rs` - gains a URL-opening helper beside the existing
  `/usr/bin/open` folder path.
- `crates/knot/src/workspace_window/` - the sidebar launcher row and the new
  content pane, following the markdown/mermaid takeover pattern.
- State refresh follows `crates/knot/src/diff_stats.rs`'s claim-refresh cache,
  which exists precisely so git subprocesses stay off the render path.
- `crates/knot-core/locales/en.yml` - the view's strings.
- New dependency: none in Rust. A new external runtime dependency on the `gh`
  CLI, which is optional - its absence degrades the feature rather than
  breaking it.

## Non-Goals

- Creating, reviewing, merging or commenting on pull requests from Knot. This
  change reads and links; it does not write to the forge.
- Forges other than GitHub. The detection pattern and the state fetch are
  GitHub-shaped. A URL Knot does not recognize is not recorded.
- Authenticating to GitHub itself. Knot uses whatever `gh` is already
  authenticated as, and reports when it is not.
- Reconstructing pull requests opened before this change shipped, or opened
  outside Knot. Only output Knot observed is recorded.
