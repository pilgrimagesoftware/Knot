# Tasks

## 1. Branch setup

- [ ] 1.1 Create the worktree/feature branch from PR #579's branch (not
  `develop` — `relative_timestamp`/`render_timestamp` don't exist on
  `develop` yet) and verify `git branch --show-current` and `git merge-base`
  confirm the expected ancestry.

## 2. Thread the hold state into the panel

- [ ] 2.1 Add `cmd_held: bool` to `panel_view::message::Message<'a>`, set by
  its caller from `workspace.key_hints.shown().is_some()`, and verify
  `cargo check -p knot` passes with the new field wired through every call
  site.
- [ ] 2.2 Gate `render_timestamp`'s call in both the `PanelMessage::User` and
  `PanelMessage::Assistant` arms of `render_message` behind
  `ctx.cmd_held`, so the `.children(...)` `Option` is `None` (no layout
  space) whenever the hold is not showing, and verify by reading the
  updated `render_message` diff against the `#577` version — the `Option`
  chain should now test `cmd_held` as well as `state.sent_at.get(index)`.

## 3. Tests

- [ ] 3.1 Add a test alongside `relative_timestamp_buckets_by_elapsed_time`
  in `message.rs`'s `#[cfg(test)] mod tests` asserting `render_message`
  with `cmd_held: false` produces no timestamp child, and with
  `cmd_held: true` produces one, and verify `cargo test -p knot
  message::tests` passes.
- [ ] 3.2 Extend `workspace_window/key_hints_tests.rs` (or add a sibling
  test) covering that a ⌘ hold recorded by `KeyHintHold` is visible to a
  panel-side reader via `shown().is_some()`, matching the sidebar's own
  assertions, and verify the test passes under `cargo test -p knot
  key_hints`.

## 4. Verification and cleanup

- [ ] 4.1 Run `make fmt-check`, `make size-check`, `make lint`, `make test`,
  `make build` and verify all five pass.
- [ ] 4.2 Manually run the app (`cargo run -p knot` or `make run` if
  defined), send a prompt, hold ⌘ over the panel, and verify timestamps
  appear after the sidebar's own key-hint delay and disappear immediately
  on release — matching this change's spec scenarios.
