# Spec Delta

## ADDED Requirements

### Requirement: Toggle Artifacts is a configurable shortcut

The configurable shortcuts SHALL include **Toggle Artifacts**, default ⌥⌘A. It
shows or hides the selected agent's artifact panel as `artifact-panel`'s "The
panel can be shown and hidden from the menu bar and the keyboard" describes.

It SHALL be a single-chord shortcut on the same terms as Jump to bottom:
- It is listed in the Keyboard settings pane.
- It is rebindable, and validated against every other configurable, fixed and
  reserved chord.
- It is stored only when customized, as `toggleArtifacts` in the preferences
  document's keybindings.

⌥⌘A puts A, for Artifacts, on the ⌥⌘ modifier the other panel toggles use. ⇧⌘A
is not available: it is the permission prompt's fixed Allow.

#### Scenario: The default toggles the panel

- **WHEN** the user has not customized Toggle Artifacts and presses ⌥⌘A with an
  agent's artifact panel shown
- **THEN** the panel is hidden

#### Scenario: A customization is stored and honoured

- **WHEN** the user rebinds Toggle Artifacts to another free chord in the
  Keyboard pane
- **THEN** the new chord toggles the panel, ⌥⌘A no longer does, and the
  preferences document stores it under `toggleArtifacts`
