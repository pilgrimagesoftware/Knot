## Why

Opening a workspace starts every agent in it at once. Each non-shell agent
spawns an adapter subprocess, connects over ACP, and burns its own model
quota on a registration turn - whether or not the user intends to work with
it today. A workspace of eight agents costs eight launches to look at one.

There is also no way to stop an agent that is running without removing it or
restarting it: "I am done with this one for now" has no expression in the
app.

## What Changes

- Agents gain an **activation mode**: `active` or `passive`.
  - An `active` agent starts when its workspace opens, which is what every
    agent does today.
  - A `passive` agent does not start until the user first selects it in the
    sidebar and its pane takes focus. It then stays running until the
    workspace (or the app) closes, or the user deactivates it.
- A new **Deactivate** item on the agent row's context menu stops a running
  agent's session without removing the agent or clearing its place in the
  workspace. Selecting the agent again starts it back up.
- The new-agent and edit-agent dialog gains an **activation control**, with a
  short hint next to it saying what the two modes mean. Agents created
  through that dialog default to `passive`.
- **Agents that already exist keep starting eagerly.** Activation mode is a
  new durable field; agents saved before this change load as `active`, so
  nobody's workspace goes quiet after an update. Only newly created agents
  default to `passive`.

## Capabilities

### New Capabilities
- `agent-editor-ui`: the new-agent/edit-agent dialog's own contract. It has
  none today - `agent-list-ui` specifies only that "Edit Agent…" opens the
  dialog, not what the dialog contains. This change gives it a home, seeded
  with the activation control and its hint; the rest of the dialog's fields
  can be written down as they are touched.

### Modified Capabilities
- `agent-lifecycle`: adds activation mode as a durable field with its
  creation default and its load default, and specifies when an agent starts
  and what deactivating it does. Today the spec covers create, restart,
  resume, edit and remove, but never says when an agent's session begins.
- `agent-list-ui`: adds the Deactivate context-menu item and the rule for
  when it is shown.

## Impact

- `knot-core`: a new durable field on the saved-agent record, defaulting to
  `active` on load so existing settings files keep their behaviour.
- `knot-agents`: the field on `Agent`, its creation default, and the runtime
  flag tracking whether an agent has been activated in this run.
- `crates/knot`: the workspace window decides what to start on open and on
  selection rather than starting everything; a placeholder pane for a
  selected-but-stopped agent; the Deactivate menu item; the editor's
  activation control and hint.
- No change to the MCP server, `knot-git`, or `knot-discovery`. A passive
  agent that has never started simply is not registered, which `list-agents`
  already reports.
