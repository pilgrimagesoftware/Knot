# Spec Delta

## MODIFIED Requirements

### Requirement: Appearance control

The window SHALL show an "Appearance" section with a picker bound to
`appearance_mode`, offering Auto, System, Light, and Dark. Changing the
selection SHALL persist the new value immediately and SHALL apply it
immediately — see `appearance-mode` for what each value resolves to and what
"apply" repaints. A persisted selection that changes nothing on screen is not
a satisfied requirement.

The section's hint SHALL describe what the control does in this app. It SHALL
NOT describe deriving the scheme from a terminal background colour, which the
port has no setting for.

#### Scenario: Changing appearance mode persists

- **WHEN** the user picks "Dark" in the Appearance picker
- **THEN** `appearance_mode` is saved as `"dark"` before the picker closes

#### Scenario: Changing appearance mode repaints

- **WHEN** the user picks "Dark" in the Appearance picker
- **THEN** the settings window and every other open window paint dark, with no
  relaunch
