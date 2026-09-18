## 1. Contract

- [x] 1.1 Add the exit-driven removal requirement to `agent-lifecycle`.

## 2. Implementation

- [x] 2.1 Confirm the existing exit hook in `workspace_window::ensure_session`
      matches the requirement. It does: the hook is registered inside
      `ensure_session`, which returns early for any agent that does not run
      a terminal process, and the drain calls `remove_agent` - the same
      `AgentStore::remove` cascade the user-initiated path uses, with no
      confirmation dialog.
- [x] 2.2 Name the scoping rule as `runs_a_terminal_process` rather than a
      bare `agent_type != "shell"` check, and test it. The removal itself
      runs inside a GPUI view and has no seam to test without a window
      harness; the rule that decides *which* agents it can ever apply to
      does.

## 3. Verification

- [x] 3.1 `make rust` passes clean.
- [ ] 3.2 Manually: open a workspace with a shell companion, type `exit` in
      its pane, and confirm the companion disappears and the layout
      collapses without a prompt.
