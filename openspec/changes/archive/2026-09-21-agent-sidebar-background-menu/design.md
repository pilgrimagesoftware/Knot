# Design

## Context

See proposal.md - Why. The mechanisms this rests on, read from the code
rather than assumed:

- The sidebar's only context menu is on the agent row `div`
  (`crates/knot/src/workspace_window/mod.rs:2842`). It is the single
  `.context_menu(` call site in `crates/knot`.
- The row menu is already split the way this one should be: a pure item-set
  function `agent_context_menu_entries(AgentMenuFacts) -> Vec<AgentMenuEntry>`
  in `app_state.rs:152`, a builder `agent_row_context_menu` that attaches
  handlers, and a dispatcher `run_agent_menu_action`. Every item is wired;
  nothing in it is a stub.
- The per-agent operations this menu fans out over all exist and are already
  called once per agent: `AgentStore::restart` (`knot-agents`
  `store/lifecycle.rs:178`), `WorkspaceWindow::remove_agent` (`:939`),
  `WorkspaceWindow::deactivate_agent` (`:996`), each followed by
  `persist_agents` where the store changed durable state.
- Confirmation dialogs open through `window.defer` then
  `window.open_alert_dialog` - deferred because a `PopupMenu` dismisses
  itself after running a handler and takes an inline dialog down with it
  (`run_agent_menu_action`, Restart/Remove arms).
- Prompt delivery to a panel agent already handles the busy case:
  `send_panel_prompt` (`:2161`) sends when the session is `Ready` and idle,
  and otherwise pushes onto `panel_prompt_queues`, which `drain_panel_prompt`
  (`:2258`) empties when the turn ends. Terminal agents have
  `TerminalSession::send_text` / `send_command` (`knot-terminal/src/lib.rs:129`).

## Goals / Non-Goals

**Goals:**

- One pure entry-set function for this menu, testable the way
  `agent_context_menu_entries` is, so its order, grouping and enablement are
  covered without a window.
- Bulk actions built from the existing per-agent operations, not from new
  store methods that would drift from them.
- Broadcast delivery that goes through the same path the agent's own composer
  uses, so queueing, recording in the conversation, and error reporting all
  behave identically.

**Non-Goals:**

- Sharing an item set with the row menu. The two menus have disjoint items
  and different enablement rules; forcing them through one enum would buy
  nothing.
- A general "bulk operation" abstraction. Three loops.

## Decisions

### A second pure entry-set function, not an extension of the first

Add `AgentListBackgroundEntry` and
`sidebar_background_menu_entries(SidebarMenuFacts) -> Vec<..>` alongside the
existing pair in `app_state.rs`, where `SidebarMenuFacts` carries
`agent_count` and `running_count`.

Unlike `agent_context_menu_entries`, this function returns every entry every
time, each paired with an enabled flag, because this menu disables rather than
omits (see the spec). That difference in return shape is the concrete reason
not to merge the two.

The `agents-menu` change in flight makes the opposite call for the menu bar -
disable rather than omit - for the same reason this menu does: a menu that
opens in a fixed place is learned by position. Consistent, not contradictory.

### The background menu goes on the list container; rows keep theirs

`.context_menu` moves onto the scrollable agent-list container, with the row's
existing `.context_menu` left where it is. gpui dispatches a right-click to
the innermost handler, so a click on a row gets the row's menu and only the
gap between and below rows reaches the container.

**Risk, to settle in task 2.1 rather than assume:** if the outer menu fires
on rows as well, or the row's suppresses the outer one over the whole list
area, the container handler must instead be attached to an explicit filler
element sized to the list's remaining space. That is a layout change, not a
requirement change.

### Bulk actions loop over the existing per-agent operations

Restart All, Close All and Deactivate All each take a snapshot of the
workspace's agent ids, then apply the same operation the row menu applies,
once per id, and persist once at the end rather than once per agent.

Snapshot first: `remove_agent` mutates the store's agent list, and iterating
a list while removing from it is the obvious way to close half a workspace.
This is also what the Swift reference does
(`let agentsToClose = agentManager.currentWorkspaceAgents` before the loop,
`SidebarView.swift:335`).

One `persist_agents` at the end rather than N: the per-agent helpers are
called directly and the persist is hoisted, which is the only place this
diverges from simply calling the row menu's handler in a loop.

Alternative considered: new `restart_all` / `remove_all` / `deactivate_all`
methods on `AgentStore`. Rejected - the teardown half of removal and
deactivation lives in the window (`teardown_session`), not the store, so a
store-level bulk method could only do half the job and would invite the two
halves to drift.

### Deactivate All is defined by the port, not ported

The Swift reference has no deactivate at any level - not per agent, not in
bulk. The port already added a per-agent Deactivate (`agent-list-ui`), and
Deactivate All is that operation applied to every running agent. It is
recorded in the spec as a deliberate divergence so a later reader does not
"correct" it against the reference.

It is also why Deactivate All carries an enablement rule the other two do not:
disabled when no agent is *running*, not merely when none exists. Restart All
and Close All apply to a stopped agent; Deactivate All does not.

### Broadcast reuses the composer's delivery path

Extract the delivery half of `send_panel_prompt` into a
`deliver_panel_prompt(&mut self, id, text)` that sends when the session is
`Ready` and idle and queues onto `panel_prompt_queues` otherwise.
`send_panel_prompt` keeps the input-reading, context-attaching and
input-clearing it does today and calls the extracted function; the broadcast
calls it directly with the sheet's text.

This is what makes the spec's queueing requirement free rather than a second
implementation of it, and it keeps `record_user_message` in one place so a
broadcast appears in each agent's conversation exactly as a typed prompt does.

For a terminal-driven agent, delivery is `TerminalSession::send_text`. An
agent with neither a panel session nor a terminal session is skipped; both
extraction points already return early in that case, so "skipped quietly" is
the existing behavior rather than a new branch.

### The broadcast sheet is modifier-Return, not Return

The field is multi-line, so Return inserts a newline and a modifier-Return
sends - the reference binds `Cmd+Return` for exactly this reason
(`SkwadApp.swift:458-503`). Escape cancels.

This is the second dialog in the app to want Return/Escape handling; the
first is the workspace name dialog in the `workspace-dialog-keyboard` change,
which is deliberately the simpler case and settles the mechanism question
(does a focused input swallow the key?) first. **This change should follow
whatever that one lands on rather than inventing a second mechanism.** If
both are in flight at once, sequence this one after it.

## Risks / Trade-offs

- **The outer context menu fires over rows.** → Settled in task 2.1 before
  the items are built; the fallback is a filler element, spelled out above.
- **Close All is destructive and irreversible.** → Confirmation naming the
  count, as the reference has, and as the row's Remove Agent already has. The
  item sits above the divider with Restart All rather than beside Broadcast,
  so the two destructive items are together and neither is adjacent to the
  harmless one.
- **A broadcast to a busy workspace queues behind every agent's current
  turn, so "sent" does not mean "seen".** → Accepted; it is the behavior the
  single-agent composer already has and the user already understands from it.
  The alternative - interrupting turns - is worse.
- **Extracting `deliver_panel_prompt` touches the composer's send path, which
  the prompt-queueing work just landed.** → Pure extraction, no behavior
  change; the existing queueing tests are the guard. Verify they still pass
  before building anything on top.
- **Three bulk actions with no undo.** → Two of the three confirm. The third,
  Deactivate All, is reversible by selecting a row, which is the same test the
  row menu's Deactivate passes.

## Open Questions

None.
