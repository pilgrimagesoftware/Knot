# Spec Delta

## ADDED Requirements

### Requirement: The two proportional font settings name what they draw

The settings surface SHALL hold two proportional font settings, each named for
the text it governs:

- The UI font (`ui_font_name` / `ui_font_size`) is the application's default
  family and text size - the face the interface and all body text is drawn in.
  It defaults to `"Adamina"` at `16.0`.
- The title font (`title_font_name` / `title_font_size`) is the family and text
  size for titles and headers. It defaults to `"Manrope"` at `14.0`.

Both SHALL be decode-tolerant: a persisted document written before these
settings existed SHALL load with each defaulted, not an error.

Neither setting SHALL be the terminal font, which stays separate
(`terminal_font_name` / `terminal_font_size`).

#### Scenario: A fresh store carries the two roles at their defaults

- **WHEN** settings are loaded with no persisted document
- **THEN** the UI font is `"Adamina"` at `16.0` and the title font is
  `"Manrope"` at `14.0`

#### Scenario: Writing one font leaves the other alone

- **WHEN** the UI font family is set to `"Iowan Old Style"`
- **THEN** that value is persisted immediately and the title font's family and
  size are unchanged

### Requirement: A pre-migration document's font roles are swapped once

Before this change the two settings held the opposite roles: the value under
`uiFontName` / `uiFontSize` was applied to titles and secondary text, and the
value under `titleFontName` / `titleFontSize` was the application-wide default.

The system SHALL record which font-role arrangement a persisted document was
written under, and SHALL migrate a document written under the old arrangement
exactly once on load by exchanging the two families' values and the two sizes'
values. A document carrying no such marker SHALL be treated as written under
the old arrangement.

The migration SHALL exchange only values the document actually carries: a
persisted font setting that is absent SHALL stay absent and take the new
default, rather than receiving the other setting's value. A document that
customized neither font therefore loads at the new defaults, unchanged in
effect.

Once migrated, the loaded settings SHALL present the new arrangement, and the
next persist SHALL record that the migration has run so it cannot run twice.

The migration SHALL NOT apply to the terminal font.

#### Scenario: Both fonts were customized

- **WHEN** a document with no migration marker holds `uiFontName` `"Helvetica
  Neue"` at size `13` and `titleFontName` `"Palatino"` at size `18` is loaded
- **THEN** the loaded UI font is `"Palatino"` at `18.0` and the loaded title
  font is `"Helvetica Neue"` at `13.0`

#### Scenario: Only one font was customized

- **WHEN** a document with no migration marker holds `titleFontName`
  `"Palatino"` and no `uiFontName`
- **THEN** the loaded UI font is `"Palatino"` and the loaded title font is the
  new default `"Manrope"`

#### Scenario: Neither font was customized

- **WHEN** a document with no migration marker holds neither font key
- **THEN** the loaded UI font is `"Adamina"` at `16.0` and the loaded title
  font is `"Manrope"` at `14.0`

#### Scenario: The migration does not run twice

- **WHEN** a document that already carries the migration marker holds
  `uiFontName` `"Adamina"` and `titleFontName` `"Manrope"`
- **THEN** the loaded values are exactly those, with no exchange

#### Scenario: A migrated document is recorded as migrated

- **WHEN** a pre-migration document is loaded and then persisted
- **THEN** the written document carries the migration marker, and loading it
  again leaves the font values as they were written
