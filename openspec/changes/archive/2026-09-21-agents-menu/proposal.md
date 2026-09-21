## Why

Every per-agent action lives behind a right-click. Edit, Fork, Duplicate,
Move to Workspace, Save to Bench, Open In, Markdown Files, Register, Restart
and Remove are reachable one way only, on a row the user has to find and
right-click, and an application with no menu for its central object is one
where a user cannot discover what they can do without guessing at the mouse.

The menu bar is also where macOS users look for what an app can do, and where
keyboard access and accessibility tooling look for it. Knot's menu bar
currently has one working item of its own ("Settings…") and a row of disabled
placeholders.

## What Changes

- Add an **Agents** menu to the application menu bar, carrying the same items
  as the agent row's context menu, in the same order and grouping.
- The menu acts on the **agent selected in the sidebar**. With no agent
  selected, its items are disabled rather than hidden.
- Items that do not apply to the selected agent are **disabled rather than
  omitted**, which is where this menu deliberately diverges from the context
  menu: a menu-bar menu whose items move between selections cannot be learned.
- The two menus SHALL be built from one item set, so neither can gain an
  item the other lacks.

## Capabilities

### New Capabilities
- `app-menu`: the application menu bar - what it offers, what each menu acts
  on, and when items are enabled. It has no contract today; `settings-ui`
  specifies only that "Settings…" appears in the app menu. This change gives
  it one, seeded with the Agents menu; the rest of the menu bar can be
  written down as it is touched.

### Modified Capabilities
(none - `agent-list-ui` keeps owning the item set, and the new capability
refers to it rather than restating it.)

## Impact

- `crates/knot`: one action type per menu item, the Agents menu built from
  `agent_context_menu_entries`, and the workspace window registering handlers
  for them while an agent is selected.
- No change to `knot-core`, `knot-agents`, or any crate below the UI. Every
  item already has a working implementation behind the context menu; this is
  a second way to reach it.
