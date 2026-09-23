# Spec Delta

## MODIFIED Requirements

### Requirement: Slash-triggered lookup
When the user types a leading `/` in the panel prompt input, the system SHALL
show a lookup popup listing the commands and skills the registry offers. The
list SHALL filter to entries whose token or description matches the text typed
after the `/`.

The slash lookup SHALL NOT be the popup's only occupant. The popup is shared
with the `@` file lookup, and the token under the caret decides which of the
two is active; the slash lookup SHALL yield when the caret sits in an `@`
token, and SHALL resume when the caret returns to a slash token.

The token the lookup inserts SHALL be one the composer's styling recognises,
so an inserted command is drawn as a token rather than as prose.

#### Scenario: Slash opens the lookup
- **WHEN** the user types `/` at the start of a line in the prompt input
- **THEN** a popup listing the available commands and skills appears above
  the input

#### Scenario: Typing filters the lookup
- **WHEN** the user types more characters after the `/`
- **THEN** the popup shows only the entries whose token or description matches
  the typed text

#### Scenario: No matches dismisses the lookup
- **WHEN** the text after the `/` matches no entry
- **THEN** the popup shows no entries and is dismissed

#### Scenario: The caret moves into an `@` token
- **WHEN** a slash lookup is open and the caret moves into an `@` token
  elsewhere in the buffer
- **THEN** the popup shows the file lookup instead, and the slash list is not
  shown alongside it

#### Scenario: An inserted command is styled as a token
- **WHEN** the lookup inserts a command into the buffer
- **THEN** the inserted range is drawn with the composer's token treatment
