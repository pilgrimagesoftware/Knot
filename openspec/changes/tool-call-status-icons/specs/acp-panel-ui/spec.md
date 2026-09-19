# Spec Delta

## MODIFIED Requirements

### Requirement: A tool call's header separates content from chrome by font

A tool call card's title - the command it ran, the path it read, whatever the
agent named the call - SHALL render in the theme's monospace family. It is a
command, a path or an identifier, and the project renders those monospace
everywhere else.

The call's status SHALL be expressed by a status icon, and the card's own
words about the call - the status word shown in the icon's tooltip - SHALL be
in the proportional family.

#### Scenario: A shell command reads as a command

- **WHEN** a tool call titled `git status --porcelain=v2` renders
- **THEN** its title is in the monospace family

#### Scenario: The status stays prose

- **WHEN** a completed tool call renders with the status word "Done" as the
  status icon's tooltip
- **THEN** that word is in the proportional family, beside a monospace title

## ADDED Requirements

### Requirement: Tool call status icon

The system SHALL render a tool call's status as an icon whose glyph and color
follow the call's state: an in-flight state (pending or running) SHALL use the
theme's info colour, a failed call the theme's danger colour, and a completed
call the same neutral muted treatment as the card's finished state. The word
for the status SHALL be carried by the icon's tooltip.

#### Scenario: A running call reads as running
- **WHEN** a tool call is `pending` or `in_progress`
- **THEN** its status icon renders in the theme's info colour and its tooltip
  says "Pending" or "Running…" as appropriate

#### Scenario: A failed call is marked
- **WHEN** a tool call reaches `failed`
- **THEN** its status icon renders in the theme's danger colour and its
  tooltip says "Failed"

#### Scenario: A completed call recedes
- **WHEN** a tool call reaches `completed`
- **THEN** its status icon renders in the neutral muted style and its tooltip
  says "Done"

#### Scenario: An unrecognized status keeps its text
- **WHEN** a tool call reports a status the panel does not recognize
- **THEN** the card shows the raw status string rather than a fabricated icon,
  matching the panel's existing pass-through behavior for unknown statuses