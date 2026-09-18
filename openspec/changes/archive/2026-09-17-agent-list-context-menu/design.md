## Context

`AgentStore` (`crates/knot-agents/src/store.rs`) already implements `edit`,
`restart`, `remove`, and `create_shell_companion`, and `agent-lifecycle`
already specifies their behavior in full. None of `edit`, `restart`,
`remove`, or `create_shell_companion` is called anywhere in `crates/knot`
today - `open_agent_editor` (`crates/knot/src/main.rs`) only ever creates a
new agent (`AgentEditor::create` calls `AgentStore::create`); there was no
edit-existing-agent path to reuse. The workspace sidebar renders each agent
as a row (`agent_rows`, built alongside `layout_model`) but has no
right-click handling at all.

Discovered while implementing this change (not part of the original
proposal): making Edit Agent... work therefore required extending
`open_agent_editor`/`AgentEditor` to be dual-purpose - an `edit_target:
Option<Uuid>` prefills the form from the existing agent and, on submit,
routes through `AgentStore::edit` instead of `AgentStore::create`. This is a
larger surface than "wire an existing call" as originally scoped, but keeps
the change self-contained rather than shipping a menu with a dead Edit
item or deferring it to a follow-up change.

`gpui-component` (the component layer `gpui-kit` re-exports) has a
right-click context-menu primitive already used elsewhere in the dependency
(`Popover`/`Tree` support `.mouse_button(MouseButton::Right)` and a
`.context_menu(...)` builder), matching the dropdown-menu pattern
`crates/knot` already uses for button-triggered menus (`Button::dropdown_menu`,
used for the appearance picker and the agent-panel-controls change's
Session Config Option selectors).

## Goals / Non-Goals

**Goals:**
- Wire a right-click context menu onto each agent-list row, reusing
  `AgentStore`'s existing `edit`/`restart`/`remove`/`create_shell_companion`
  as-is - no `AgentStore` API changes.
- Follow `knot-ui-conventions`' destructive-action rule: Restart and Remove
  both confirm via `window.open_alert_dialog` before applying.
- Match the Swift reference's companion-awareness: a shell companion's menu
  omits New Shell Companion and Restart Agent (a companion can't own
  companions or restart independently of its owner, per `agent-lifecycle`).

**Non-Goals:**
- No new `AgentStore` capabilities - Duplicate Agent, Move to Workspace, Save
  to Bench, Register Agent, and the Markdown Files / Open In... submenus from
  the Swift reference are left for a later change, since their backends are
  either unported (`AgentStore` has no duplicate or bench-save method) or
  need their own design pass (multi-workspace UI, external-app launching).
- No change to `AgentStore::remove`/`restart`/`create_shell_companion`
  themselves - this change is UI wiring onto already-specified behavior.

## Decisions

- **Confirm dialog copy**: "Remove Agent" and "Restart Agent" each get their
  own `window.open_alert_dialog` prompt naming the agent (e.g. "Remove
  <name>? This closes its terminal session."), matching the existing
  destructive-confirm pattern elsewhere in `crates/knot` (persona delete,
  workspace delete) rather than a generic "Are you sure?".
- **Companion gating in the menu builder, not in `AgentStore`**: the
  New-Shell-Companion/Restart visibility rule is a UI concern (which buttons
  to show), computed from `Agent.is_companion` at menu-build time - it does
  not change what `AgentStore::create_shell_companion`/`restart` themselves
  accept or reject.
- **No optimistic UI**: each action calls straight into the existing
  synchronous `AgentStore` methods (already used by other call sites in
  `crates/knot`) and triggers a re-render via the window's normal `cx.notify()`
  path - no new async/queued state.

## Risks / Trade-offs

- Right-click affordances have no visible indicator in this UI today (no
  "..." menu button as a discoverability fallback) - if user testing shows
  people don't find it, a small overflow-menu button per row is a natural
  follow-up, not blocking this change.
- `AgentStore::remove`'s cascade (removing owned companions, tearing down
  terminal sessions, re-selecting panes) is already covered by
  `agent-lifecycle`'s existing tests; this change adds no new coverage there,
  only for the new UI entry point (menu visibility, confirm-dialog wiring).
