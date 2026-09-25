# Spec Delta

## ADDED Requirements

### Requirement: The View menu lists the navigation shortcuts

The View menu SHALL carry an item for each workspace navigation shortcut in
`keybindings`, in this order, above the Enter Full Screen item macOS adds:

| Item | Key |
| --- | --- |
| View > Dashboard | the Toggle Dashboard shortcut (default ⌥⌘O) |
| View > Pull Requests | the Toggle Pull Requests shortcut (default ⌥⌘P) |
| separator | |
| View > Focus Agent Input | the Focus agent input shortcut (default ⌘L) |
| View > Jump to Bottom | the Jump to bottom shortcut (default ⌃⌘↓) |
| separator | |
| View > Select Agent ▸ | submenu, see "The View menu's Select Agent submenu" |
| View > Select Workspace ▸ | submenu, see "The View menu's Select Workspace submenu" |

Choosing an item SHALL do exactly what its shortcut does (`keybindings`). Each
item SHALL show the shortcut's current binding and SHALL update when the user
rebinds it, without a restart.

Open Command Center SHALL NOT appear in the View menu; it stays in the Window
menu. The View menu SHALL still carry exactly one Enter Full Screen item, per
"Standard menu items carry the platform's key equivalents".

This diverges from the Swift reference, whose View menu holds Toggle Git
Panel, Toggle Sidebar, Detach Workspace, Cycle Workspace and Next/Previous
Agent. The port has none of those as shortcuts, and lists the ones it has.

#### Scenario: The items and their keys

- **WHEN** the user opens the View menu with a workspace window focused and no
  shortcut customized
- **THEN** it lists Dashboard (⌥⌘O), Pull Requests (⌥⌘P), Focus Agent Input
  (⌘L), Jump to Bottom (⌃⌘↓), Select Agent and Select Workspace, in that order,
  followed by one Enter Full Screen item

#### Scenario: An item does what its shortcut does

- **WHEN** the agent view is showing and the user chooses View > Pull Requests
- **THEN** the Pull Requests panel is shown, the same as pressing ⌥⌘P

#### Scenario: The menu follows a rebinding

- **WHEN** the user rebinds Focus agent input to ⌃⌘I and opens the View menu
- **THEN** View > Focus Agent Input shows ⌃⌘I

### Requirement: View menu enablement follows the focused window

The Dashboard, Pull Requests, Focus Agent Input, Jump to Bottom and Select
Agent items SHALL be enabled only while a workspace window is focused, because
their shortcuts act only on that window. Beyond that:

- Focus Agent Input SHALL be disabled when the window has no agent selected.
- Jump to Bottom SHALL be disabled when the window has no agent selected, when
  the selected agent is in terminal mode, and while a Dashboard or Pull
  Requests panel is showing - the cases in which its shortcut does nothing.

Select Workspace SHALL be enabled whenever at least one workspace exists,
including with no Knot window open, because its shortcut works from anywhere.

A disabled item SHALL still show its key equivalent.

#### Scenario: No workspace window focused

- **WHEN** the Command Center is focused and the user opens the View menu
- **THEN** Dashboard, Pull Requests, Focus Agent Input, Jump to Bottom and
  Select Agent are disabled, and Select Workspace is enabled

#### Scenario: Jump to Bottom for a terminal agent

- **WHEN** a terminal-mode agent is selected and the user opens the View menu
- **THEN** Jump to Bottom is disabled and Focus Agent Input is enabled

#### Scenario: Jump to Bottom with a panel showing

- **WHEN** a panel-mode agent is selected, the Dashboard is showing, and the
  user opens the View menu
- **THEN** Jump to Bottom is disabled

### Requirement: The View menu's panel items show which panel is showing

View > Dashboard SHALL be checked while the focused workspace window shows the
Dashboard, and View > Pull Requests SHALL be checked while it shows the Pull
Requests panel. Neither SHALL be checked while the agent view is showing or
when no workspace window is focused.

#### Scenario: The Dashboard is showing

- **WHEN** the Dashboard is showing and the user opens the View menu
- **THEN** Dashboard is checked and Pull Requests is not

#### Scenario: Back to the agent view

- **WHEN** the user presses ⌥⌘O twice and opens the View menu
- **THEN** neither Dashboard nor Pull Requests is checked

### Requirement: The View menu's Select Agent submenu

View > Select Agent SHALL list the first nine agents in the focused workspace
window's sidebar, top to bottom, each by name. The Nth item SHALL show the
Select agent N shortcut (default ⌥⌘N) and choosing it SHALL do what that
shortcut does. The selected agent's item SHALL be checked.

The submenu SHALL reflect the sidebar's current agents, names and order each
time it is opened, including agents added, removed, renamed or reordered since
launch. With no workspace window focused, or an empty workspace, the submenu
SHALL be disabled.

#### Scenario: Selecting an agent from the menu

- **WHEN** a workspace window lists agents X, Y and Z and the user chooses
  View > Select Agent > Y
- **THEN** Y is selected and shown, the same as pressing ⌥⌘2, and Y's item
  shows ⌥⌘2

#### Scenario: An agent added since the bar was built

- **WHEN** the user adds agent W to the focused workspace and opens View >
  Select Agent
- **THEN** W is listed in its sidebar position

#### Scenario: More than nine agents

- **WHEN** the focused workspace has twelve agents
- **THEN** View > Select Agent lists the first nine

### Requirement: The View menu's Select Workspace submenu

View > Select Workspace SHALL list the first nine workspaces in the order the
workspace manager lists them, each by name. The Nth item SHALL show the Select
workspace N shortcut (default ⌘N) and choosing it SHALL do what that shortcut
does. The item for the focused workspace window's workspace SHALL be checked.

The submenu SHALL reflect the current workspaces, names and order each time it
is opened, including workspaces created, removed, renamed or reordered since
launch.

#### Scenario: Opening a workspace from the menu

- **WHEN** workspace C has no open window and the user chooses View > Select
  Workspace > C
- **THEN** C's window opens, the same as pressing its ⌘N shortcut

#### Scenario: With no window open

- **WHEN** every Knot window is closed and the user opens View > Select
  Workspace
- **THEN** the workspaces are listed and enabled

#### Scenario: A renamed workspace

- **WHEN** the user renames workspace B to Backend and opens View > Select
  Workspace
- **THEN** the item reads Backend

### Requirement: File > New Agent opens the agent editor

The File menu SHALL carry a New Agent… item, directly below New Workspace, with
the fixed key equivalent ⌘T, the key the Swift reference uses for it. Choosing
the item or pressing ⌘T SHALL open the agent editor for a new agent in the
focused workspace window, the same dialog the sidebar's New agent control
opens. It SHALL work while a Dashboard or Pull Requests panel is showing.

The item SHALL be enabled only while a workspace window is focused, and ⌘T
SHALL do nothing in any other window. ⌘T is a fixed shortcut, so a navigation
shortcut customized to ⌘T SHALL be rejected like any other conflict with a
fixed shortcut (`keybindings`).

#### Scenario: New agent by keyboard

- **WHEN** a workspace window is focused and the user presses ⌘T
- **THEN** the agent editor opens for a new agent in that workspace

#### Scenario: From a panel

- **WHEN** the Pull Requests panel is showing and the user chooses File > New
  Agent…
- **THEN** the agent editor opens for a new agent in that workspace

#### Scenario: No workspace window focused

- **WHEN** the workspace manager is focused and the user opens the File menu
- **THEN** New Agent… is disabled and shows ⌘T

#### Scenario: Customizing onto ⌘T

- **WHEN** the user records ⌘T for Focus agent input
- **THEN** the recording is rejected as a conflict with New Agent
