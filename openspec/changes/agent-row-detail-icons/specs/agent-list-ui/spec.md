## ADDED Requirements

### Requirement: Every detail line on an agent row is labelled by an icon

An agent row's detail lines - the ones under the agent's name - SHALL each
carry a leading icon identifying what that line is: its agent type, its
persona, its status, and its folder.

Without one, a line is a bare string whose meaning has to be inferred from
its content, which fails exactly when it matters: an agent whose status text
happens to look like a path, or whose folder is named after a person, reads
as the wrong thing entirely.

The icons SHALL be drawn as icons rather than as characters inside the line's
text, so they share the lines' muted colour, size with their line, and sit in
one column down the row.

#### Scenario: A status line is identifiable as a status

- **WHEN** an agent row renders with a status
- **THEN** that line carries a leading icon marking it as the status

#### Scenario: A folder line is identifiable as a folder

- **WHEN** an agent row renders its folder
- **THEN** that line carries a leading icon marking it as a folder

#### Scenario: The icons line up

- **WHEN** an agent row renders with a type, a persona, a status and a folder
- **THEN** all four leading icons share the row's muted colour and align in
  one column

#### Scenario: A line with nothing to show has no icon

- **WHEN** an agent has no persona
- **THEN** no persona line and no persona icon are drawn, rather than an icon
  beside an empty line

### Requirement: A detail line's icon is sized to its own line

Each detail line's icon SHALL be sized to that line's text, not to a single
fixed size across the row. The row's lines do not all render at the same text
size, and an icon fixed to one of them reads as undersized or oversized
beside the others.

#### Scenario: Icons match their lines

- **WHEN** a row renders detail lines at more than one text size
- **THEN** each line's icon matches the text beside it
