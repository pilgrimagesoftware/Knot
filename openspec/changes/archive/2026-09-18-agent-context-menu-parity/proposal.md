## Why

`agent-list-ui` specifies four context-menu items - Edit Agent, New Shell
Companion, Restart Agent, Remove Agent. The Swift reference
(`Skwad/Views/Components/AgentContextMenu.swift`) has twelve, plus its own
divider layout and a set of visibility rules. The eight missing ones are
the only route to several operations the port already implements and
otherwise cannot reach from the UI at all: moving an agent between
workspaces, forking or duplicating it, saving it to the bench, and
registering it with MCP by hand.

The hand-off from `acp-only-agent-launch` recorded this as the last
UI-parity gap on that branch, and judged it too large for a patch: most of
the backing exists, but the menu needs visibility rules, submenus and
confirmations that belong in the contract rather than in a commit message.

## What Changes

- Extend the agent row context menu to the Swift reference's full item
  set, in its order, with its dividers.
- Add the visibility and enablement rules that decide which items a given
  row shows - the Swift `AgentMenuVisibility` behaviour - rather than the
  port's current single companion/non-companion split.
- Add the two submenus the reference has: the workspace list for moving an
  agent, and the application list for opening its folder elsewhere.
- Keep every destructive item behind the confirmation the reference uses.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-list-ui`: extends the agent row context menu with the reference's
  remaining items, its visibility rules, and its submenus.

## Impact

- `crates/knot` (`workspace_window`, `agent_editor`): menu construction,
  the new item handlers, and the submenu rendering.
- `crates/knot-agents`: reuses existing store operations where they exist;
  adds one only where the menu needs an operation the store cannot express.
- No changes to `knot-git`, `knot-discovery`, or the MCP server's wire
  protocol.
