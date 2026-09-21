# Spec Delta

## ADDED Requirements

### Requirement: A font picker renders in the font it names

Each row's control in the Appearance tab's Fonts section SHALL render its
label in the family that label names, so the section shows three faces rather
than three strings in the same face.

The label SHALL keep its existing text - the family name and the size in
points - and SHALL render at the settings window's own text size, not at the
configured point size. The point size is already stated as a number; drawing
the label at it would let one row's setting change every row's height.

#### Scenario: A row shows its own face

- **WHEN** the UI font is set to one family and the Terminal font to another
- **THEN** each row's label is drawn in its own family, and the two differ

#### Scenario: Picking a font updates the face immediately

- **WHEN** the user picks a new family in the font panel opened from a row
- **THEN** that row's label is redrawn in the newly chosen family, without
  reopening the settings window

#### Scenario: The point size does not change the label's size

- **WHEN** the Title font's size is set to 48
- **THEN** the Title row's label reads "48pt" and is drawn at the same text
  size as the UI and Terminal rows, leaving all three rows the same height

### Requirement: An unresolvable font is marked rather than substituted

A persisted family that the text system cannot resolve - one uninstalled
since it was chosen, or one the OS font panel accepts that the app's own font
lookup does not - SHALL be shown in the default face and marked as
unavailable, in the row itself, so the mismatch between the name and the face
is stated rather than left for the user to notice.

The row SHALL go on naming the persisted family rather than the substitute.
What the user needs is to see that their choice is not in effect; renaming the
row to the fallback would hide exactly that.

The persisted value SHALL NOT be rewritten. A font that is missing today may
be installed tomorrow, and silently replacing the setting would lose a choice
the user made deliberately.

#### Scenario: A font that is not installed

- **WHEN** the UI font is set to a family that is not installed
- **THEN** the UI row still names that family, is drawn in the default face,
  and is marked as unavailable
- **AND** `ui_font_name` still holds that family

#### Scenario: A resolvable font is not marked

- **WHEN** every configured family resolves
- **THEN** no row carries the unavailable marking

#### Scenario: Installing the font clears the marking

- **WHEN** a row is marked unavailable and the named family becomes
  resolvable
- **THEN** the marking goes and the row renders in that family, with no
  change to the persisted value
