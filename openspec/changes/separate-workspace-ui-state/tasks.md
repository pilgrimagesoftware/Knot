# Tasks

## 1. Capture the upgrade case before changing anything

- [x] 1.1 Capture a real combined `workspaces.json` from an installation — one
      with several workspaces, at least one moved, split and detached — and
      commit it as a test fixture; verify it holds all eight UI fields on at
      least one record
- [x] 1.2 Write the migration test against that fixture first, asserting field
      by field that every configured value lands in the workspaces document and
      every arrangement value in the UI-state document. It fails until section 3
      lands; that is the point — a migration written before its test is a
      migration whose silent-loss mode nobody checks

## 2. The records and the document

- [x] 2.1 Add `WorkspaceUiState` to `crates/knot-core/src/settings/records.rs`
      holding the eight fields with their current serde defaults, and remove
      them from `Workspace`; verify with `cargo build -p knot-core`
- [x] 2.2 Add the document: a constant beside the other filenames, a
      `StorePaths` accessor, and `Settings` holding
      `BTreeMap<Uuid, WorkspaceUiState>` with a `persist_workspace_ui` writer
      that writes that document alone; verify with `cargo test -p knot-core`
- [x] 2.3 Update the document table in `store.rs`'s module doc and the
      two-directory description in `store/paths.rs`'s module doc — both state
      the old layout in prose and are part of the change; verify by reading them
      back against the new set of documents
- [x] 2.4 Prune entries with no matching workspace on load, in one place, with a
      comment saying why pruning is on load rather than on deletion; verify with
      a test that loads a UI-state document holding an orphan

## 3. The migration

- [x] 3.1 Split a combined workspaces document on load: read it as a `Value`,
      detect the UI keys, write the UI-state document first (existing entries
      winning), then rewrite the workspaces document without them; verify with
      the 1.2 test
- [x] 3.2 Add a test for the crash-part-way case — UI-state document already
      written, workspaces document still combined — asserting the second load
      keeps the written entries and finishes; verify with
      `cargo test -p knot-core`
- [x] 3.3 Add a test for the two-step upgrade: a legacy `settings.json`
      installation arrives at separate preferences, collection and UI-state
      documents with arrangement preserved. This is the path no one will try by
      hand; verify with `cargo test -p knot-core`
- [x] 3.4 Add tests for a missing UI-state document and for one undecodable
      entry among decodable ones, asserting workspaces still load and only
      arrangement is lost; verify with `cargo test -p knot-core`

## 4. The runtime store and its callers

- [ ] 4.1 Add the parallel UI map to `AgentStore` and move
      `set_workspace_window_bounds` and the other UI-state setters onto it in
      `crates/knot-agents/src/store/workspace.rs`; verify with
      `cargo test -p knot-agents`
- [ ] 4.2 Add the map to whatever teardown already prunes per-key state, so the
      two pruning rules cannot disagree; verify with `make lint`
- [ ] 4.3 Follow the compiler through the call sites in `crates/knot` — around
      100 references across the eight fields — changing reads and writes to go
      through the UI state and changing nothing else. Any improvement noticed on
      the way belongs in a separate change; verify with `make build`
- [ ] 4.4 Update the `Workspace` literals in `crates/knot/src/workspace_manager/mod.rs`,
      `crates/knot/src/tests/mod.rs` and `crates/knot/src/tests/import_window.rs`;
      verify with `cargo test -p knot`
- [x] 4.5 Check `crates/knot-core/src/import/workspaces.rs`: an imported
      workspace is configuration, so confirm it creates no UI-state entry and
      that an imported workspace opens with default arrangement; verify with
      `cargo test -p knot-core import`

## 5. The payoff

- [ ] 5.1 Change the bounds observer in `crates/knot/src/workspace_window/open.rs`
      to write the UI-state document alone, not through `persist_agents` and not
      through the window's `Settings` snapshot; verify by dragging a window and
      confirming `agents.json` and `workspaces.json` are byte-for-byte unchanged
      while `workspace-ui-state.json` updates
- [ ] 5.2 Add a note to issue #238 recording that this removed the most frequent
      trigger but not the stale snapshot itself, so the bug is now rarer and
      harder to reproduce rather than fixed

## 6. Walk the spec

- [ ] 6.1 Upgrade in place: run a build with a combined document present,
      confirm windows open where they were left, then confirm the workspaces
      document holds only configured fields
- [ ] 6.2 Delete `workspace-ui-state.json` and start the app; confirm every
      workspace loads with its name, color and agents, and windows open with
      default arrangement
- [ ] 6.3 Delete a workspace, reload, and confirm no UI-state entry survives for
      it
- [ ] 6.4 Confirm arrangement still round-trips: move, resize, split and detach a
      workspace's window, restart, and check it comes back the same
- [ ] 6.5 Run `make` and confirm the whole gate passes — `fmt-check`,
      `size-check`, `clippy -D warnings`, tests, build
