# Spec Delta

## ADDED Requirements

### Requirement: View > Artifacts

The View menu SHALL list an **Artifacts** item directly after Jump to Bottom and
before the separator above Select Agent. It dispatches the Toggle Artifacts
shortcut's action, and its key equivalent is that shortcut's binding, default
⌥⌘A. Its enablement comes from the owning workspace window registering the
action's handler, as the other navigation items' does. It is enabled only while
the selected agent has an artifact panel shown, or one closed that can be
reopened. It is checked while the panel is shown.

#### Scenario: The item is disabled without an artifact

- **WHEN** the owning window's selected agent has never had an artifact
- **THEN** View > Artifacts is disabled

#### Scenario: The item carries the shortcut

- **WHEN** the View menu is built with the default shortcuts
- **THEN** Artifacts shows ⌥⌘A beside its label
