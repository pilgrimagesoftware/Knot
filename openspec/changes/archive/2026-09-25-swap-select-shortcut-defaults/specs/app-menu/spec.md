## MODIFIED Requirements

### Requirement: The View menu's Select Agent submenu

View > Select Agent SHALL list the first nine agents in the focused workspace
window's sidebar, top to bottom, each by name. The Nth item SHALL show the
Select agent N shortcut (default ⌘N) and choosing it SHALL do what that
shortcut does. The selected agent's item SHALL be checked.

The submenu SHALL reflect the sidebar's current agents, names and order each
time it is opened, including agents added, removed, renamed or reordered since
launch. With no workspace window focused, or an empty workspace, the submenu
SHALL be disabled.

#### Scenario: Selecting an agent from the menu

- **WHEN** a workspace window lists agents X, Y and Z and the user chooses
  View > Select Agent > Y
- **THEN** Y is selected and shown, the same as pressing ⌘2, and Y's item
  shows ⌘2

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
workspace N shortcut (default ⌥⌘N) and choosing it SHALL do what that shortcut
does. The item for the focused workspace window's workspace SHALL be checked.

The submenu SHALL reflect the current workspaces, names and order each time it
is opened, including workspaces created, removed, renamed or reordered since
launch.

#### Scenario: Opening a workspace from the menu

- **WHEN** workspace C has no open window and the user chooses View > Select
  Workspace > C
- **THEN** C's window opens, the same as pressing its ⌥⌘N shortcut

#### Scenario: With no window open

- **WHEN** every Knot window is closed and the user opens View > Select
  Workspace
- **THEN** the workspaces are listed and enabled

#### Scenario: A renamed workspace

- **WHEN** the user renames workspace B to Backend and opens View > Select
  Workspace
- **THEN** the item reads Backend
