# Tasks

## 1. URL detection

- [ ] 1.1 Add `crates/knot-core/src/pull_request_url.rs` with a
      `PullRequestUrl { host, owner, repo, number }` type and
      `scan_pull_request_urls(&str)`; verify unit tests cover a bare URL, a
      `/files` suffix, a `#issuecomment-…` fragment, an issues URL, a repo root
      URL, a non-GitHub host, and several URLs in one string
- [ ] 1.2 Give `PullRequestUrl` a canonical `Display` that drops any suffix, so
      two spellings of one pull request compare equal; verify a test asserting
      the `/files` and bare forms produce the same canonical string
- [ ] 1.3 Add a byte-stream scanner with a carry buffer for chunk boundaries;
      verify a test that feeds one URL one byte at a time still yields it, and
      one that feeds 1 MB of unrelated output yields nothing

## 2. Persistence

- [ ] 2.1 Add a `SavedPullRequest` record (url, agent id, workspace id,
      first-seen) to `crates/knot-core/src/settings/records.rs`; verify a
      serde round-trip test
- [ ] 2.2 Add the sixth collection document to the store's document table and
      its read/write path in `crates/knot-core/src/settings/store/`; verify a
      test that writing a pull request leaves `agents.json` untouched
- [ ] 2.3 Verify a store written before this document loads with no recorded
      pull requests and writes no document until one is recorded; add that test
- [ ] 2.4 Add the collection and its mutators to `crates/knot-agents/src/store/`
      - record (idempotent per agent), remove, list by workspace, list by agent;
      verify tests for idempotence and for the per-agent duplicate case
- [ ] 2.5 Cascade removal from the existing agent-removal and
      workspace-removal paths; verify tests that removing each drops its
      records

## 3. Forge client

- [ ] 3.1 Create the `knot-forge` crate with a `ForgeError` thiserror enum, a
      `Result` alias and a `consts.rs`, wired into the workspace and the root
      `Cargo.toml` dependency list; verify `cargo build --workspace` succeeds
- [ ] 3.2 Implement `probe() -> ForgeAvailability` (`Missing | Unauthenticated |
      Ready`) over `gh auth status`; verify tests with a stubbed runner for all
      three outcomes
- [ ] 3.3 Implement `pull_request_state(url)` over
      `gh pr view <url> --json number,title,state,isDraft,statusCheckRollup`
      with the 30s timeout; verify tests parsing a real captured payload for
      open, draft, merged and closed, and one with an unknown extra field
- [ ] 3.4 Verify a missing field leaves that part of the state absent rather
      than failing the parse; add that test

## 4. Detection taps

- [ ] 4.1 Scan `ToolCallContent::Text` in `crates/knot/src/panel_state/` as it
      is folded in, recording through the store; verify a test that applies a
      `ToolCallResult` carrying a URL and asserts one record
- [ ] 4.2 Scan the PTY byte stream in `knot-terminal` before it reaches the
      grid, recording through the same call; verify a test that feeds a URL
      through the transport and asserts one record
- [ ] 4.3 Verify the same URL arriving on both paths for one agent produces one
      record; add that test
- [ ] 4.4 Benchmark the terminal scan against a flood of output and verify it
      does not measurably slow the grid feed; record the numbers in the PR

## 5. State cache

- [ ] 5.1 Add a claim-refresh cache for pull request state following
      `crates/knot/src/diff_stats.rs` - per-URL TTL, writer, dirty flag -
      either by generalizing that type or as a sibling; verify unit tests for
      claim, TTL expiry and dirty-flag handoff
- [ ] 5.2 Drive refreshes from `WorkspaceWindow`'s tokio runtime via
      `spawn_blocking`, only while the Pull Requests view is showing; verify no
      `gh` process is spawned while the view is closed
- [ ] 5.3 Verify a row keeps its last state while a refresh is in flight rather
      than blanking; add that test against the cache

## 6. The view

- [ ] 6.1 Add the Pull Requests launcher row to the sidebar in
      `crates/knot/src/workspace_window/render/sidebar.rs`, with its state
      breakdown and selected background; verify the count updates when a record
      is added with the window open
- [ ] 6.1a Derive the breakdown from the state cache - open (draft included),
      merged, closed, and pending for records whose state is not known; verify
      unit tests for a fully-known set, a fully-unknown set falling back to the
      total, and a mixed set counting the rest as pending
- [ ] 6.1b Verify the breakdown follows a state change on the next refresh, and
      that it needs no `gh` process while the view is closed - the row falls
      back to the total instead; add both tests
- [ ] 6.2 Add the content pane listing the workspace's pull requests grouped by
      agent, newest first, following the markdown/mermaid takeover in
      `workspace_window/panel/pane.rs`; verify the pane replaces the content and
      the row toggles back
- [ ] 6.3 Render each row's fetched state - number, title, draft/open/merged/
      closed, check rollup - and the empty-workspace message; verify against a
      workspace with and without records
- [ ] 6.4 Show the single availability message derived from `probe()`, wording
      distinguishing missing, unauthenticated and failed; verify each of the
      three by stubbing the probe
- [ ] 6.5 Add `open_url` to `crates/knot/src/open_in.rs` beside `run_open` and
      wire the row click to it; verify clicking a row opens the browser, and
      that a row without state is still clickable
- [ ] 6.6 Add row removal and verify a removed row stays absent after a restart
- [ ] 6.7 Add the view's strings to `crates/knot-core/locales/en.yml` and route
      every user-facing string through `l10n::t`; verify the key test resolves
      them, touching `knot-core` first so the build is not stale

## 7. Gate

- [ ] 7.1 Run `make` and verify the full gate passes - fmt, size-check, lint,
      test, build
- [ ] 7.2 Walk the scenarios in `specs/pull-request-tracking/spec.md` against
      the running app, including the `gh`-absent and `gh`-unauthenticated paths,
      and verify each holds
