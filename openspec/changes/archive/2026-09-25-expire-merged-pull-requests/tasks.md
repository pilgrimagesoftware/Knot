# Tasks

## 1. Fetch the merge time

- [x] 1.1 Add `time.workspace = true` to `crates/knot-forge/Cargo.toml` and verify `cargo build -p knot-forge` succeeds
- [x] 1.2 Add `mergedAt` to `PULL_REQUEST_FIELDS` in `crates/knot-forge/src/consts.rs` and extend its doc comment to say the field feeds the expiry rule rather than a row, so the "nothing a row renders" principle still reads true; verify by inspecting the constant
- [x] 1.3 Add `merged_at: Option<String>` to `RawPullRequest` and `merged_at: Option<OffsetDateTime>` to `PullRequestState` in `crates/knot-forge/src/pull_request.rs`, parsing with `time`'s `Rfc3339` and yielding `None` on an absent or unparseable value; verify `cargo test -p knot-forge` passes
- [x] 1.4 Add a `mergedAt` value to `crates/knot-forge/src/pull_request/testdata/merged-passing.json`; verify the existing merged-state parse test still passes and now observes the timestamp
- [x] 1.5 Add parse tests in `crates/knot-forge/src/pull_request/tests.rs` covering: merged with a timestamp yields `Some`; merged with the field absent yields `None`; merged with an unparseable value yields `None`; open and closed yield `None`. Verify `cargo test -p knot-forge` passes

## 2. Prune records in the store

- [x] 2.1 Add a predicate-driven removal to `AgentStore` in `crates/knot-agents/src/store/pull_requests.rs`, beside `remove_pull_request` and the `forget_*` cascades, that drops the records whose URLs the caller names and reports whether anything was removed; verify `cargo test -p knot-agents` passes
- [x] 2.2 Add store tests in `crates/knot-agents/src/store/tests/pull_requests.rs` covering: naming no URLs removes nothing and reports false; naming one URL removes that sighting for every agent that recorded it and leaves the rest; naming an unrecorded URL removes nothing. Verify `cargo test -p knot-agents` passes

## 3. Decide expiry and wire it in

- [x] 3.1 Add `PULL_REQUEST_MERGED_RETENTION: Duration = Duration::from_secs(24 * 60 * 60)` to `crates/knot/src/consts.rs` beside the existing pull request intervals, with a doc comment giving the rationale; verify by inspecting the constant
- [x] 3.2 Add a pure function to `crates/knot/src/pull_request_state.rs` that, given the state cache snapshot, the workspace's recorded URLs, a retention window and a "now", returns the URLs to expire - merged, merge time known, and elapsed since the merge greater than the window; verify `cargo test -p knot` passes
- [x] 3.3 Add unit tests for that function in `crates/knot/src/pull_request_state/tests.rs` covering: merged past the window expires; merged within the window is kept; merged with no merge time is kept; an unfetched URL is kept; a failed fetch is kept; open, draft and closed are kept whatever their age; a merge time in the future is kept. Verify `cargo test -p knot` passes
- [x] 3.4 Call the expiry check from `refresh_pull_request_states` in `crates/knot/src/workspace_window/pull_requests_view.rs` so it runs only while the Pull Requests view is shown, and on a non-empty result run the existing store-mutate -> `persist_pull_requests` -> `prune_pull_request_states` sequence, with no confirmation dialog; verify `cargo test -p knot` passes
- [x] 3.5 Confirm the frame does no work when nothing has expired - the empty result path must not persist, must not touch the state cache and must not read a file - and cover it with a test that a refresh over non-expiring records performs no write; verify `cargo test -p knot` passes

## 4. Integration behavior

- [x] 4.1 Add a test in `crates/knot/src/tests/pull_request_records.rs` that a merged, past-window record is gone from `pull_requests_for_workspace` and from the persisted document after a refresh, and that the pull request on the forge is untouched; verify `cargo test -p knot` passes
- [x] 4.2 Add a test that the launcher row's counts drop the expired record - merged count down by one, total down by one - without the view being reopened, exercising `counts_for` over the post-prune record list; verify `cargo test -p knot` passes
- [x] 4.3 Add a test that a URL recorded again after expiring reappears in the list and expires again on the next refresh that resolves it as merged and past the window; verify `cargo test -p knot` passes

## 5. Documentation and gate

- [x] 5.1 Update the `pull-request-tracking` module docs in the touched crates so they point at the requirements this change adds, per the project's convention that module docs link back to the contract; verify by inspecting the doc comments
- [x] 5.2 Run `openspec validate expire-merged-pull-requests --strict` and verify it reports the change valid
- [x] 5.3 Run `make` and verify the whole gate passes - `fmt-check`, `size-check`, `lint` with `-D warnings`, `test` and `build` - splitting any file the 700-line limit catches rather than raising the limit
