# Tasks

## 1. Group by URL

- [x] 1.1 Add `crates/knot/src/pull_request_groups.rs` with a pure
      `group_records` that collapses a workspace's records by URL, groups URLs by
      their owning agent set, orders shared groups first and rows newest first
      by earliest sighting. Verify `cargo test -p knot` passes.
- [x] 1.2 Add `crates/knot/src/pull_request_groups/tests.rs` covering: a
      single-agent URL stays in its agent's group; a URL two agents recorded is
      one row in one group headed by both; shared groups come first; a shared
      row orders by its earliest sighting; two URLs with the same owners share a
      group. Verify `cargo test -p knot` passes.

## 2. Read through it

- [x] 2.1 De-duplicate `workspace_pull_request_urls` in
      `workspace_window/pull_requests_view.rs`, preserving newest-first order.
- [x] 2.2 Build `pull_request_groups` from `group_records`; `PullRequestGroup`
      carries the owning agent ids and names; `render_row` takes the owning ids.
- [x] 2.3 Removing a row removes the record for every owning agent.
- [x] 2.4 Add a test in `crates/knot/src/pull_request_groups/tests.rs` that a
      pull request six agents recorded counts once in `counts_for`, through
      `unique_urls`, which `workspace_pull_request_urls` now delegates to.

## 3. Contract

- [x] 3.1 Modify "The workspace window lists its pull requests" and add "A pull
      request several agents opened is listed once". Verify
      `openspec validate shared-pull-request-rows --strict` passes.

## 4. Gate

- [x] 4.1 Run `make` and confirm the full gate passes.
