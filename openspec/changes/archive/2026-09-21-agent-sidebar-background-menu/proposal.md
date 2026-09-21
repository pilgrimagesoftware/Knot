# Proposal

## Why

Right-clicking an agent row in the workspace sidebar opens a menu of thirteen
actions. Right-clicking the empty space below the rows opens nothing. Every
action that applies to the workspace's agents *as a set* - add one, restart
them all, close them all, send them all the same message - is therefore
unreachable from the place a user naturally reaches for it, and three of the
four have no other route at all.

The Swift reference puts exactly this menu on the sidebar's scroll area
(`Skwad/Views/Sidebar/SidebarView.swift:127-158`). The port has no
`.context_menu` on anything but the agent row itself.

## What Changes

- Right-clicking the empty area of the workspace sidebar's agent list SHALL
  open a background context menu scoped to the workspace rather than to any
  one agent.
- The menu carries **New Agent**, **Restart All**, **Close All**,
  **Deactivate All**, a divider, and **Broadcast to All Agents…**.
- Restart All, Close All and Deactivate All SHALL be disabled when the
  workspace has no agents - not hidden. Unlike the row menu, which opens on a
  row and omits what does not apply, this menu always opens on the same empty
  space and is learned by position.
- Restart All and Close All SHALL each ask for confirmation naming the number
  of agents affected, as the reference does. Deactivate All SHALL NOT, matching
  the row menu's Deactivate: nothing is lost that selecting a row will not
  bring back.
- Broadcast to All Agents… SHALL open a multi-line message sheet and, on send,
  deliver the typed text to every agent in the workspace the same way the user
  typing into each agent would - a prompt to an ACP panel session, injected
  text to a terminal agent.

**Deactivate All has no Swift precedent.** The reference has no deactivate
concept at any level. The port does: `agent-list-ui` already specifies a
per-agent Deactivate, and this change defines the bulk form as that action
applied to every running agent in the workspace.

**Broadcast does not go through MCP.** `mcp-messaging` already owns a
`broadcast` that fans a message out to an agent's inbox for it to read with
`check-messages`. That is agent-to-agent and deliberately different: this menu
item is the user speaking to their agents, and the reference implements it as
text injection (`sendBroadcast`, `SidebarView.swift:312-319`), not as a
queued message. This change does not touch `mcp-messaging`.

## Capabilities

### Modified Capabilities

- `agent-list-ui`: adds the sidebar background context menu - its items,
  their order and grouping, their enablement, which of them confirm, and what
  each does. The capability already owns the sidebar's per-agent row menu;
  this is the same surface's other menu.

### New Capabilities

(none)

## Impact

- `crates/knot`: a second pure entry-set function alongside
  `agent_context_menu_entries` in `app_state.rs`, a `.context_menu` on the
  sidebar list container in `workspace_window/mod.rs`, bulk handlers reusing
  the existing `remove_agent` / `deactivate_agent` / `store.restart` paths,
  and a broadcast message sheet.
- `crates/knot-agents`: no new store operations expected - `restart`,
  `remove` and `deactivate` already exist and are already called per-agent.
- No change to `knot-messaging` or `knot-mcp-tools`.

## Non-Goals

- A menu-bar equivalent. The in-flight `agents-menu` change covers the row
  menu's items; whether these workspace-wide items belong there too is a
  question for that change, not this one.
- Keyboard shortcuts for any of these items.
- Acting on agents outside the current workspace. Every item here is scoped
  to the workspace the sidebar is showing, as the reference's
  `currentWorkspaceAgents` is.
