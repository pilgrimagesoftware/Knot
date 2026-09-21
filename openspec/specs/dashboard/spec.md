# dashboard Specification

## Purpose

Defines the card-grid view of agents: the in-place dashboard a workspace
window toggles to in place of its terminal view, the separate Command Center
window that shows the same grid across workspaces, how each agent's card is
laid out and sorted, and the tile that creates a new agent from the grid.

## Requirements

### Requirement: Dashboard launcher

The workspace window SHALL show the dashboard launcher as a labelled row at
the top of its agent list - an icon and the word "Dashboard" - shaped like
the agent rows it sits above. Clicking it toggles the workspace window's own
content between its terminal view and the dashboard view; it does not open a
separate window. The row SHALL show a selected background while the dashboard
view is the one being shown, as an agent row does when selected.

It is deliberately not an icon-only button in the bottom bar beside "New
agent". The dashboard is the workspace's overview of every agent, so it
belongs above them and shaped like them; as a bare icon among the window's
controls it read as a minor one and went unnoticed. This follows the Swift
reference's `overviewRow` (`Skwad/Views/Sidebar/SidebarView.swift`).

#### Scenario: The launcher sits above the agent rows

- **WHEN** a workspace window is open
- **THEN** a labelled "Dashboard" row is visible at the top of the agent
  list, above the first agent row, and not in the bottom bar beside "New
  agent"

#### Scenario: The launcher marks which view is showing

- **WHEN** the workspace window is showing the dashboard view
- **THEN** the Dashboard row is drawn with a selected background
- **WHEN** it is showing the terminal view
- **THEN** the Dashboard row is drawn without one

#### Scenario: Launcher toggles the in-place view

- **WHEN** a workspace window is open and the Dashboard row is clicked
- **THEN** that window's content switches from the terminal view to the
  dashboard view, scoped to that workspace's agents
- **AND** clicking a card, or clicking the row again, switches back to
  the terminal view

### Requirement: Command Center window

A separate, global dashboard window ("Command Center") SHALL show every
attached workspace's agents, reusing the same agent card grid as the
workspace-scoped dashboard.

#### Scenario: Command Center shows all workspaces

- **WHEN** the Command Center window is open
- **THEN** every attached workspace appears as its own section in the
  grid, not just one workspace

#### Scenario: Card in Command Center opens the workspace window

- **WHEN** an agent's card is clicked in the Command Center
- **THEN** that agent's workspace window opens (or focuses, if already
  open) showing its terminal view with that agent selected

### Requirement: Agent card grid

The dashboard SHALL show agents grouped by workspace, each in a card
showing its avatar, name, status text and color, folder name, and git
diff stats, with an "Add Agent" tile per workspace group.

#### Scenario: Card reflects agent state

- **WHEN** an agent's automatic state is `Running`
- **THEN** its card shows the orange status indicator and the current
  status/title text

#### Scenario: Card shows git diff stats

- **WHEN** an agent's folder has uncommitted changes
- **THEN** its card shows insertions/deletions/file counts parsed via
  `knot_git::parse_numstat`

#### Scenario: Empty workspace

- **WHEN** a workspace has no non-companion agents
- **THEN** its section shows "No agents" instead of a card grid

### Requirement: Sort modes

The dashboard SHALL support sorting each workspace's agent cards by
manual (store order), name, or status.

#### Scenario: Sort by name

- **WHEN** the user selects "Name" in the sort picker
- **THEN** each workspace's cards are ordered alphabetically by agent name

### Requirement: Add Agent tile

Each workspace section SHALL show an "Add Agent" tile that opens the
existing agent-creation dialog, prefilled for that workspace.

#### Scenario: Add Agent opens prefilled dialog

- **WHEN** the user clicks "Add Agent" in a workspace's section
- **THEN** the agent editor opens with the folder prefilled from an
  existing agent in that workspace, if any
