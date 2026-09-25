# Spec Delta

## ADDED Requirements

### Requirement: Holding ⌘ shows the sidebar's shortcuts

While the user holds ⌘ in a focused workspace window, the sidebar SHALL show
the key that reaches each of these controls, beside it:

| Control | Hint |
| --- | --- |
| Dashboard row | the Toggle Dashboard shortcut (default ⌥⌘O) |
| Pull Requests row | the Toggle Pull Requests shortcut (default ⌥⌘P) |
| The Nth agent row, N from 1 to 9 | the Select agent N shortcut (default ⌘N) |
| New agent control | the New Agent shortcut (⌘T) |

Each hint SHALL show the binding currently in effect, in the glyph form macOS
menus use, and SHALL follow a rebinding without a restart. Agent rows after
the ninth have no shortcut and SHALL show no hint.

The hints SHALL appear only after ⌘ has been held for 500 ms with no other key
pressed, so that an ordinary ⌘ shortcut such as ⌘C does not flash them. They
SHALL disappear as soon as ⌘ is released, a non-modifier key is pressed, or the
window stops being the key window. Adding ⌥, ⌃ or ⇧ while ⌘ is held SHALL NOT
hide them, because the panel shortcuts need ⌥.

Hints are shown whether or not the binding includes ⌘. ⌘ is the trigger
because every default binding holds it.

A hint SHALL NOT move or resize the control it labels, or any other row: at
full width it takes the right end of the row, over the row's own trailing
content. In the compact layout it SHALL be drawn as a badge over the corner of
the avatar or icon.

The hints are not a Swift reference feature. The Swift sidebar shows no
shortcuts.

#### Scenario: Hints appear while ⌘ is held

- **WHEN** a workspace window lists agents X, Y and Z, no shortcut is
  customized, and the user holds ⌘ for a second
- **THEN** the Dashboard row shows ⌥⌘O, the Pull Requests row ⌥⌘P, X ⌘1, Y
  ⌘2, Z ⌘3, and the new agent control ⌘T

#### Scenario: A quick shortcut does not flash them

- **WHEN** the user presses ⌘C and releases both keys within 200 ms
- **THEN** no hint appears

#### Scenario: Releasing ⌘ hides them

- **WHEN** the hints are showing and the user releases ⌘
- **THEN** every hint disappears

#### Scenario: Adding ⌥ keeps them

- **WHEN** the hints are showing and the user also presses ⌥
- **THEN** the hints stay

#### Scenario: Switching away hides them

- **WHEN** the hints are showing and the user presses ⌘Tab to another
  application
- **THEN** no hint remains when the user returns to the window

#### Scenario: A rebinding shows

- **WHEN** the user rebinds Toggle Dashboard to ⌃⌘D and holds ⌘
- **THEN** the Dashboard row shows ⌃⌘D

#### Scenario: A tenth agent

- **WHEN** a workspace has ten agents and the user holds ⌘
- **THEN** the first nine rows show a hint and the tenth shows none

#### Scenario: Compact layout

- **WHEN** the sidebar is compact and the user holds ⌘
- **THEN** each hint is a badge over its avatar or icon, and no row changes size

#### Scenario: Another window

- **WHEN** the Command Center is focused and the user holds ⌘
- **THEN** no workspace window's sidebar shows hints
