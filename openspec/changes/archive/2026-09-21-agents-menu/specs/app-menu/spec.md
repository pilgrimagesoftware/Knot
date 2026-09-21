## Purpose

Defines the application menu bar: which menus it carries, what each acts on,
and when their items are enabled. Distinct from the in-app context menus,
which act on whatever the pointer is over.

## ADDED Requirements

### Requirement: Agents menu

The menu bar SHALL carry an Agents menu offering the same items as the agent
row's context menu, in the same order and with the same grouping, per
`agent-list-ui`. Both menus SHALL be produced from one item set, so an item
added to either appears in both.

The menu SHALL act on the agent currently selected in the focused workspace
window's sidebar, and selecting an item SHALL do exactly what the same item
does from that agent's context menu, including any confirmation it asks for.

#### Scenario: The menu offers what the context menu offers

- **WHEN** the user opens the Agents menu with an agent selected
- **THEN** it lists the same items, in the same order and groups, as that
  agent's context menu

#### Scenario: An item does the same thing from either menu

- **WHEN** the user selects Remove Agent from the Agents menu
- **THEN** the same confirmation appears, and confirming removes the same
  agent, as selecting Remove Agent from that agent's row

### Requirement: Agents menu enablement follows the selection

Every item in the Agents menu SHALL be disabled when no agent is selected in
the focused workspace window, and SHALL become enabled as soon as one is.

An item that does not apply to the selected agent SHALL be shown disabled
rather than omitted. This differs from the context menu, which omits such
items: a context menu is read fresh at the pointer each time, while a menu
bar is navigated from memory, and items that move or vanish between
selections cannot be learned.

#### Scenario: No agent selected

- **WHEN** the user opens the Agents menu with no agent selected - on the
  dashboard, or in a workspace whose selection was cleared
- **THEN** every item is present and disabled

#### Scenario: An item that does not apply to this agent

- **WHEN** the user opens the Agents menu with a shell companion selected
- **THEN** Fork Agent, Duplicate Agent and Register Agent are present and
  disabled, in their usual positions

#### Scenario: Selecting an agent enables the menu

- **WHEN** the user selects an agent in the sidebar
- **THEN** the Agents menu's items that apply to it become enabled without
  the user reopening the window

#### Scenario: No workspace window

- **WHEN** no workspace window is focused - only the workspace manager is
  open
- **THEN** every item in the Agents menu is disabled

### Requirement: Agents menu submenus reflect current state

The Agents menu's submenus - the workspaces an agent can move to, and the
agent's markdown files - SHALL list what is current for the selected agent at
the time the menu is opened, not what was current when the window opened.

#### Scenario: A workspace added since launch

- **WHEN** the user creates a second workspace and then opens the Agents
  menu on an agent in the first
- **THEN** Move to Workspace offers the new workspace

#### Scenario: A markdown file shown since launch

- **WHEN** an agent displays a markdown file and the user then opens the
  Agents menu on that agent
- **THEN** Markdown Files lists it
