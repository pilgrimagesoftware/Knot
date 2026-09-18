## 1. Context menu scaffold

- [ ] 1.1 Locate the agent-row rendering (`agent_rows`, alongside
      `layout_model` in `crates/knot/src/main.rs`) and attach a right-click
      context menu per row, scoped to that row's agent id. Verify:
      `cargo build -p knot` succeeds and right-clicking a row opens an
      (initially empty or placeholder) menu in a live run.
- [ ] 1.2 Build the menu's item set per row from `Agent.is_companion`: Edit
      Agent... always; New Shell Companion and Restart Agent only when
      `!is_companion`; Remove Agent always. Verify: unit test covers both
      the companion and non-companion item sets.

## 2. Edit Agent

- [ ] 2.1 Wire Edit Agent... to the existing `open_agent_editor` call already
      used elsewhere in `crates/knot/src/main.rs`, passing the row's agent
      id. Verify: manual run opens the editor pre-populated for that agent.

## 3. New Shell Companion

- [ ] 3.1 Wire New Shell Companion to `AgentStore::create_shell_companion`
      for the row's agent id, then `cx.notify()`. Verify: manual run creates
      a companion placed after its owner in a split layout.

## 4. Restart Agent

- [ ] 4.1 Wire Restart Agent to prompt via `window.open_alert_dialog` (name
      the agent in the copy) before calling `AgentStore::restart`. Verify:
      unit test asserts `restart` is not called until the prompt is
      confirmed; manual run confirms cancel leaves the agent running
      unchanged.

## 5. Remove Agent

- [ ] 5.1 Wire Remove Agent to prompt via `window.open_alert_dialog` (name
      the agent in the copy) before calling `AgentStore::remove`. Verify:
      unit test asserts `remove` is not called until the prompt is
      confirmed; manual run confirms cancel leaves the agent in place and
      confirm tears down its terminal session and any owned companions.

## 6. Verification pass

- [ ] 6.1 Run `make rust` (fmt + clippy + test + build) and fix any
      failures introduced by this change. Verify: command exits 0.
- [ ] 6.2 Manual pass against `specs/agent-list-ui/spec.md`'s scenarios
      (companion vs. non-companion menu, edit, new companion, restart
      confirm/cancel, remove confirm/cancel) in a live run.
