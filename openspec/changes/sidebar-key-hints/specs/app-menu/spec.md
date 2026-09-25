# Spec Delta

## ADDED Requirements

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
