# Spec Delta

## MODIFIED Requirements

### Requirement: Collapsed tool call header is a single line

A collapsed tool call SHALL render its header on a single line: the title
SHALL truncate with an ellipsis when it does not fit rather than wrap, and
the status indicator SHALL remain visible on that same line. An expanded tool
call SHALL NOT be affected.

A title containing line breaks SHALL still render as one line. The header SHALL
join it - each run of whitespace read as a single space - and then truncate it
to the header's width, so a multi-line shell command reads as the beginning of
that command rather than as every line of it. Declaring the line unwrappable is
not sufficient: text is shaped one line per line break whether wrapping is
permitted or not.

The joined line SHALL keep the title's leading content: what identifies a call
is how its command starts, so truncation takes from the end.

#### Scenario: A long title ellipsizes when collapsed

- **WHEN** a collapsed tool call's title is longer than the header's width
- **THEN** the header is one line tall and the title ends in an ellipsis

#### Scenario: The indicator stays on the line

- **WHEN** a collapsed tool call's title is ellipsized
- **THEN** the status indicator remains visible at the row's end rather than
  being pushed off or wrapped

#### Scenario: Expanded cards are unchanged

- **WHEN** the user expands a tool call whose title is long
- **THEN** the header behaves as it does today and the title is free to wrap

#### Scenario: A multi-line command collapses to one line

- **WHEN** a collapsed tool call's title is a shell command spanning several
  lines, such as a heredoc
- **THEN** the header is one line tall, reading the start of that command with
  its line breaks shown as spaces and ending in an ellipsis

#### Scenario: An expanded card keeps the agent's own line breaks

- **WHEN** the user expands a tool call whose title spans several lines
- **THEN** the title is shown as the agent sent it, its line breaks intact

#### Scenario: A short multi-line title needs no ellipsis

- **WHEN** a collapsed tool call's title has a line break but is short enough
  to fit once joined
- **THEN** the header is one line tall, shows the whole title, and carries no
  ellipsis

## ADDED Requirements

### Requirement: A row that declares one line renders one line

Wherever the panel renders text on a single line - a collapsed tool call's
title, a queued prompt's row - it SHALL render one line regardless of what the
text contains, including text the user or the agent supplied with line breaks in
it.

Such a row SHALL NOT grow to the height of its text. A queued multi-line prompt
is a row in a list of what is waiting, and a row whose height is set by its
content displaces the conversation around it.

The text itself SHALL NOT be altered: joining line breaks is how the row is
drawn, and what is stored, delivered to the agent, or shown when the same text
is rendered somewhere that permits multiple lines SHALL be unchanged.

#### Scenario: A queued multi-line prompt occupies one row

- **WHEN** the user enqueues a prompt written across several lines
- **THEN** its queued row is one line tall, its line breaks shown as spaces and
  its text ellipsized if it does not fit

#### Scenario: The delivered prompt is not the flattened one

- **WHEN** that queued prompt is delivered to the agent
- **THEN** the agent receives the prompt as the user wrote it, line breaks
  included

#### Scenario: The conversation shows the prompt in full

- **WHEN** that prompt has been delivered and appears as a message in the
  conversation
- **THEN** it is shown as written, across as many lines as it needs
