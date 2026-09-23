# Tasks

## 1. The shared handle

- [x] 1.1 Promote `arc-swap` to a direct `[workspace.dependencies]` entry at the
      version already resolved in `Cargo.lock`, add it to `knot-core`, and
      verify `cargo build --workspace` succeeds with no new lockfile churn.
- [x] 1.2 Add a shared settings handle to `knot-core` wrapping
      `ArcSwap<Settings>`: a read returning `Arc<Settings>`, and a
      read-modify-swap write that applies a closure to a clone of the current
      value and installs it. Verify with a test that a write is visible to a
      handle read afterwards and invisible to an `Arc` taken before it.
- [x] 1.3 Make the write a compare-and-retry (`rcu`-style) rather than a bare
      store, so two writers cannot drop each other's change. Verify with a
      test that interleaves two writes of different fields and asserts both
      survive.
- [x] 1.4 Register the handle as a GPUI global in `knot`, alongside
      `WindowRegistry` and `QuitGuard`, seeded from the `Settings::load()` that
      `app_bootstrap` already performs. Verify the app builds and starts with
      nothing yet reading the global.

## 2. Move the writers

- [x] 2.1 Move `knot-core`'s `import/personas.rs` off whole-surface `persist()`
      onto the personas document, taking the current value through the handle
      rather than a caller's `&mut Settings`. Verify with a test that changes a
      scalar between opening the import and running it, and asserts the scalar
      survives.
- [x] 2.2 Do the same for `import/workspaces.rs`, writing the roster documents
      only. Verify with the equivalent test, and that imported workspaces and
      agents still read back.
- [ ] 2.3 Move `settings_window`'s persist path onto the handle and delete its
      owned `settings` field. Verify the existing settings-window tests pass
      unchanged.
- [ ] 2.4 Move `workspace_window/sidebar_layout.rs` onto the handle and delete
      its hand-written `Settings::load()` guard. Verify the sidebar width still
      persists and that a preference written elsewhere is not reverted by a
      width write.

## 3. Move the readers

- [ ] 3.1 Convert `workspace_window`'s reads to the handle and delete its
      `settings` field, taking one `Arc<Settings>` per render rather than one
      per read. Verify `crates/knot/src/tests/settings_reach_open_windows.rs`
      still passes against the new path.
- [ ] 3.2 Convert `workspace_manager` (`state.rs`) and delete its `settings`
      field. Verify a preference changed while the manager is open is visible
      to it without reopening, which it was not before.
- [x] 3.3 Convert `import_window` and delete its `settings` field and its
      `Settings::load()` at open. Verify the spec's "A non-workspace window
      sees the change too" scenario.
- [ ] 3.4 Convert `command_center` and delete its `settings` field. Verify the
      command centre builds and its existing tests pass.
- [ ] 3.5 Convert `agent_editor` and delete its `settings` field and its
      `Settings::load()` guard. Verify agent creation and editing still
      persist.
- [ ] 3.6 Convert the `agent_row` menu targets and delete both
      `Settings::load()` guards there. Verify the agent row menu actions still
      persist the roster.
- [ ] 3.7 Rework `app_bootstrap` to seed the global once and stop threading
      `Settings` by value through window constructors. Verify the app starts
      and every window opens.
- [ ] 3.8 Decide `knot-mcp-tools`'s `with_settings`: either take the handle or
      keep an owned value if it is genuinely a one-shot. Verify its tests pass
      and record which it is in the module doc.

## 4. Delete the machinery the copies required

- [ ] 4.1 Delete `crates/knot/src/settings_broadcast.rs` and its call in
      `settings_window`'s persist. Verify nothing references it and the build
      is clean.
- [ ] 4.2 Delete `Settings::reload_preferences`, its `#[serde(skip)]`
      transplant and the test that holds the two in step. Verify no caller
      remains.
- [ ] 4.3 Confirm no struct field anywhere holds a `Settings` or an
      `Arc<Settings>`: grep the workspace and verify the only holders are
      locals and the global.

## 5. Verification

- [ ] 5.1 Add a test for each scenario in the delta spec that is not already
      covered by the tasks above, in particular "Two windows writing different
      values" and "A reader mid-frame sees one consistent set".
- [ ] 5.2 Confirm no settings read on the render path acquires a lock: verify
      by inspection that the render path takes one `Arc` and reads from it.
- [ ] 5.3 Run `make` and confirm formatting, size check, lint and the full test
      suite pass. Split any file the conversion pushes over 700 lines.
