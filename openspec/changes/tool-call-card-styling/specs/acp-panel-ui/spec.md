## ADDED Requirements

### Requirement: A tool call's header separates content from chrome by font

A tool call card's title - the command it ran, the path it read, whatever the
agent named the call - SHALL render in the theme's monospace family. It is a
command, a path or an identifier, and the project renders those monospace
everywhere else.

The call's status text SHALL render in the proportional family. It is the
panel's own words about the call, not anything the agent produced, and it
SHALL NOT be swept into monospace along with the title.

#### Scenario: A shell command reads as a command

- **WHEN** a tool call titled `git status --porcelain=v2` renders
- **THEN** its title is in the monospace family

#### Scenario: The status stays prose

- **WHEN** a completed tool call renders with the status text "Done"
- **THEN** that text is in the proportional family, beside a monospace title

### Requirement: A tool call's outline says what state it is in

A tool call card's outline SHALL be coloured by its status:

- **failed**: the theme's danger colour.
- **in progress** (pending or running): the theme's info colour.
- **completed**: the same neutral border every other card in the panel uses.

A completed call SHALL NOT be given a colour of its own. Success is the
common case, and a conversation whose every finished call is outlined in
green is one where nothing stands out - which defeats the outline's only
purpose, marking the two states worth looking at.

Colours SHALL come from the theme rather than fixed values, so the card
follows a theme change like the rest of the panel.

#### Scenario: A failed call is marked

- **WHEN** a tool call reaches `failed`
- **THEN** its outline is the theme's danger colour

#### Scenario: A running call is marked

- **WHEN** a tool call is `pending` or `in_progress`
- **THEN** its outline is the theme's info colour

#### Scenario: A completed call recedes

- **WHEN** a tool call reaches `completed`
- **THEN** its outline is the panel's neutral card border, the same as an
  untouched card

#### Scenario: A call's outline follows it through its lifecycle

- **WHEN** a call starts, runs and then completes
- **THEN** its outline is the info colour while it runs and the neutral
  border once it is done, without the card being rebuilt around it
