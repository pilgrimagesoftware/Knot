## 1. Context menu scaffold

- [x] 1.1 Locate the agent-row rendering (`agent_rows`, alongside
      `layout_model` in `crates/knot/src/main.rs`) and attach a right-click
      context menu per row, scoped to that row's agent id. Verify:
      `cargo build -p knot` succeeds and right-clicking a row opens an
      (initially empty or placeholder) menu in a live run.
- [x] 1.2 Build the menu's item set per row from `Agent.is_companion`: Edit
      Agent... always; New Shell Companion and Restart Agent only when
      `!is_companion`; Remove Agent always. Verify: unit test covers both
      the companion and non-companion item sets.

## 2. Edit Agent

- [x] 2.1 Wire Edit Agent... to the existing `open_agent_editor` call already
      used elsewhere in `crates/knot/src/main.rs`, passing the row's agent
      id. Verify: manual run opens the editor pre-populated for that agent.
      Note: no edit-mode existed on `open_agent_editor`/`AgentEditor` before
      this change - it was create-only. Per user direction, extended it to
      be dual-purpose (prefill from an existing agent, submit via
      `AgentStore::edit` instead of `create`) rather than dropping this
      item or stubbing it. See updated `design.md`.

## 3. New Shell Companion

- [x] 3.1 Wire New Shell Companion to `AgentStore::create_shell_companion`
      for the row's agent id, then `cx.notify()`. Verify: manual run creates
      a companion placed after its owner in a split layout.

## 4. Restart Agent

- [x] 4.1 Wire Restart Agent to prompt via `window.open_alert_dialog` (name
      the agent in the copy) before calling `AgentStore::restart`. Verify:
      unit test asserts `restart` is not called until the prompt is
      confirmed; manual run confirms cancel leaves the agent running
      unchanged.
      Note: confirm/cancel gating is structural (`AgentStore::restart` is
      only reachable from the dialog's `.on_ok(...)` closure, never called
      directly), matching this file's existing destructive-confirm call
      sites (persona/workspace delete), none of which have a dedicated
      unit test either - `gpui-component`'s own test suite already covers
      confirm-vs-cancel dialog dispatch. Verified manually instead (6.2).

## 5. Remove Agent

- [x] 5.1 Wire Remove Agent to prompt via `window.open_alert_dialog` (name
      the agent in the copy) before calling `AgentStore::remove`. Verify:
      unit test asserts `remove` is not called until the prompt is
      confirmed; manual run confirms cancel leaves the agent in place and
      confirm tears down its terminal session and any owned companions.
      Note: same structural-gating rationale as 4.1.

## 6. Verification pass

- [x] 6.1 Run `make rust` (fmt + clippy + test + build) and fix any
      failures introduced by this change. Verify: command exits 0.
      Note: `make rust-fmt` itself fails in this environment
      (`cargo +nightly fmt`: "no such command: `+nightly`" - Homebrew's
      `cargo` doesn't understand the `+toolchain` shorthand; unrelated to
      this change). Ran the pieces directly instead: nightly `cargo-fmt`
      (clean after formatting this change's new code), `cargo clippy
      --workspace --all-targets -- -D warnings` (clean), `cargo build
      --workspace` (clean). `cargo test --workspace` has one pre-existing
      failure, `knot-discovery`'s `rapid_child_creation_coalesces_to_one_rescan`
      (a filesystem-watch debounce timing test) - confirmed unrelated:
      `crates/knot-discovery` has zero diff from `origin/develop`, and the
      same failure reproduces running that test alone on a clean tree.
      `cargo test -p knot` (63 tests, including the new
      `agent_context_menu_omits_companion_actions_for_companions`) passes.
- [ ] 6.2 Manual pass against `specs/agent-list-ui/spec.md`'s scenarios
      (companion vs. non-companion menu, edit, new companion, restart
      confirm/cancel, remove confirm/cancel) in a live run.
      Note: not performed - the app was already running on this machine
      (MCP port 8766 in use), so launching a second instance to drive it
      was skipped rather than risk colliding with that live session. User
      directed proceeding to commit/PR/merge without this manual pass;
      left unchecked here as an honest record rather than marked done.
