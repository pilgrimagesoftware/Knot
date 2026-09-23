# Spec Delta

## ADDED Requirements

### Requirement: The stored mode resolves to the painted appearance

The system SHALL resolve `Settings::appearance_mode` and the appearance
reported by the operating system to one effective appearance, and SHALL paint
that:

- `Light` SHALL resolve to light, whatever the OS reports.
- `Dark` SHALL resolve to dark, whatever the OS reports.
- `System` SHALL resolve to whatever the OS reports.
- `Auto` SHALL resolve to whatever the OS reports.

`Auto` and `System` therefore behave identically. The Swift reference derives
`auto` from the terminal background colour's luminance; the Rust port has no
terminal background setting, so there is no luminance to read. Both variants
remain because `Auto` is the stored default and dropping it would change the
meaning of an existing settings file. When the port gains a configurable
terminal background, `Auto` is where that derivation belongs.

Resolution SHALL be a pure function of those two inputs, so it is decidable
without a running window.

#### Scenario: An explicit choice overrides the OS

- **WHEN** `appearance_mode` is `Dark` and the OS reports light
- **THEN** the app paints dark

#### Scenario: System follows the OS

- **WHEN** `appearance_mode` is `System` and the OS reports dark
- **THEN** the app paints dark

#### Scenario: Auto follows the OS

- **WHEN** `appearance_mode` is `Auto` and the OS reports light
- **THEN** the app paints light

### Requirement: The resolved appearance is applied at startup

The system SHALL apply the resolved appearance when the application starts,
before the first window is shown, so a stored choice is visible in the first
frame rather than after a repaint.

#### Scenario: A stored choice survives a relaunch

- **WHEN** the user picked Light, quit, and relaunched on a Mac set to dark
- **THEN** the app opens light

### Requirement: An OS appearance change does not overwrite an explicit choice

When the operating system's appearance changes under any window, the system
SHALL re-resolve through the stored mode rather than adopt the OS appearance
directly.

An OS flip therefore repaints the app only when the mode defers to the OS
(`System` or `Auto`). Under `Light` or `Dark` the app SHALL keep painting the
chosen appearance.

#### Scenario: A flip repaints under System

- **WHEN** `appearance_mode` is `System` and the OS switches from light to dark
- **THEN** the app repaints dark

#### Scenario: A flip is ignored under an explicit choice

- **WHEN** `appearance_mode` is `Light` and the OS switches from light to dark
- **THEN** the app keeps painting light

### Requirement: Changing the mode repaints every open window

When the appearance mode changes, the system SHALL re-resolve and repaint
every open window, without requiring a relaunch and without the user having to
focus each window in turn.

The system palette laid over the theme SHALL be re-applied after the theme is
re-resolved, because re-resolving reloads the light or dark configuration
wholesale and discards that overlay.

#### Scenario: Picking a mode repaints immediately

- **WHEN** the user picks Dark with a workspace window and the settings window
  both open
- **THEN** both windows repaint dark without a relaunch

#### Scenario: The palette survives the repaint

- **WHEN** the appearance is re-resolved for any reason
- **THEN** the accent and neutral colours drawn from the system palette are
  present in the theme afterwards
