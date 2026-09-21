# Spec Delta

## MODIFIED Requirements

### Requirement: Appearance tab

The Appearance tab SHALL show a "Fonts" section with one row per
configurable font — UI, Title and Terminal — bound to `ui_font_name` /
`ui_font_size`, `title_font_name` / `title_font_size`, and
`terminal_font_name` / `terminal_font_size` respectively.

Each row SHALL govern the text its label names:

- The UI row SHALL govern the application's default family and text size — the
  face the interface and all body text is drawn in, including Markdown body
  text.
- The Title row SHALL govern titles and headers, including Markdown headers.
- The Terminal row SHALL govern embedded terminal text only.

A row SHALL NOT be bound to a setting that governs other text than the row's
label names. A user changing the font the app is written in looks under UI, and
a row that renamed itself to whatever field it happened to write would send
them to the wrong one.

Each row SHALL offer a single control naming the current family and size,
which opens the OS font panel pre-selected to that family and size. One
control picks both, because the panel carries its own size field; the tab
SHALL NOT show a separate numeric size field. A choice made in the panel
SHALL persist immediately.

The tab SHALL NOT show a terminal engine picker or color pickers.

#### Scenario: Changing the font name persists

- **WHEN** the user picks "JetBrains Mono" in the font panel opened from the
  Terminal row
- **THEN** `terminal_font_name` is saved as `"JetBrains Mono"` immediately

#### Scenario: Changing the font size persists

- **WHEN** the user sets the size to `14` in the font panel opened from the
  Terminal row
- **THEN** `terminal_font_size` is saved as `14.0` immediately

#### Scenario: Each row drives its own font

- **WHEN** the user picks a family in the font panel opened from the UI row
- **THEN** `ui_font_name` is saved and `title_font_name` and
  `terminal_font_name` are unchanged

#### Scenario: The UI row changes the face the interface is drawn in

- **WHEN** the user picks a family in the font panel opened from the UI row
- **THEN** the interface's own text — window chrome, labels, buttons, dialogs,
  and Markdown body text — is drawn in that family

#### Scenario: The Title row changes headers rather than the interface

- **WHEN** the user picks a family in the font panel opened from the Title row
- **THEN** titles and headers are drawn in that family and the rest of the
  interface is unchanged
