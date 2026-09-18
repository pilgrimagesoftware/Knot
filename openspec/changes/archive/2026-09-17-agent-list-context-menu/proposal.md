## Why

`agent-lifecycle` already specifies edit, restart, remove, and shell-companion
creation, and `AgentStore` already implements all four (`edit`, `restart`,
`remove`, `create_shell_companion`) - but nothing in `crates/knot` calls
`restart`, `remove`, or `create_shell_companion` from the UI. Today the only
reachable per-agent action is opening the agent editor. There is no way to
close/remove an agent, restart a stuck one, or add a shell companion without
directly editing persisted settings - the Swift reference
(`AgentContextMenu.swift`) exposes all of these via a right-click context menu
on each agent-list row.

## What Changes

- Add a right-click context menu on each agent row in the workspace sidebar,
  triggered the same way other native context menus in `crates/knot` are
  (`gpui_kit`'s context-menu primitive), scoped to the row under the pointer.
- Menu items, gated by whether the agent is itself a shell companion (a
  companion can't own companions or be restarted independently, per
  `agent-lifecycle`):
  - **Edit Agent...** - opens the existing agent editor dialog.
  - **New Shell Companion** (non-companion agents only) - calls
    `AgentStore::create_shell_companion`.
  - **Restart Agent** (non-companion agents only) - calls `AgentStore::restart`,
    with the destructive-confirm treatment `knot-ui-conventions` requires.
  - **Remove Agent** - calls `AgentStore::remove`, with the same confirm
    treatment; removing a non-companion also removes any companions it owns
    (already `AgentStore::remove`'s behavior).
- Out of scope for this change (left for a follow-up, since their backend
  isn't ported yet or needs its own design pass): Duplicate Agent, Move to
  Workspace, Save to Bench, Register Agent, Markdown Files submenu, Open
  In... - all present in the Swift reference's `AgentContextMenu` but not
  requested here.

## Capabilities

### New Capabilities
- `agent-list-ui`: the workspace sidebar's per-agent row interactions - the
  right-click context menu and the actions it exposes.

### Modified Capabilities
(none - `agent-lifecycle`'s edit/restart/remove/companion-creation behavior is
unchanged; this change only adds a UI entry point that already-specified
`AgentStore` operations were missing.)

## Impact

- `crates/knot` (GPUI Kit UI shell): extends the agent-row rendering (near
  `layout_model`/the sidebar's `agent_rows` construction in `main.rs`) with a
  context-menu attachment and the three action handlers, plus a confirm
  dialog for the two destructive actions.
- No changes to `knot-agents`, `knot-core`, `knot-git`, `knot-discovery`, or
  the MCP server - `AgentStore`'s existing methods are reused as-is.
