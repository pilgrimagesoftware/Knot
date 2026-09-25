# Spec Delta

## MODIFIED Requirements

### Requirement: Window scope

The settings window SHALL show a tab strip with eight tabs — General,
Coding, Personas, Autopilot, Voice, MCP, Appearance, Keyboard — in that order,
with General selected by default when the window opens. Every tab SHALL render
its real pane; none render a placeholder. No "check for updates" control
SHALL be shown anywhere in the window.

The Keyboard tab's contents are specified by `keybindings`. The Swift reference
has no Keyboard tab.

#### Scenario: General is the default tab

- **WHEN** the settings window opens
- **THEN** the General tab is selected and its four sections (Appearance,
  Startup, Notifications, Agent Panel) are visible

#### Scenario: Keyboard is the last tab

- **WHEN** the settings window opens
- **THEN** Keyboard is the eighth tab, after Appearance
