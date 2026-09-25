# Spec Delta

## MODIFIED Requirements

### Requirement: Lookup entry registry
The lookup SHALL draw its entries from a registry offering the panel's built-in
commands, the skills available to the selected agent, and the prompts in the
prompt library. Each entry SHALL carry a token and a description. A library
prompt's entry SHALL use the prompt's name as its token and a single-line
preview of its text as its description, and SHALL be distinguishable in the
popup from commands and skills. The registry SHALL be extensible without
changes to the lookup popup.

#### Scenario: Built-in commands are listed
- **WHEN** the slash lookup is open on a panel agent
- **THEN** the popup lists the built-in panel commands known to the app

#### Scenario: Agent skills are listed
- **WHEN** the selected agent has skills available
- **THEN** those skills appear in the slash lookup alongside the built-in
  commands

#### Scenario: A skill root that cannot be read is not an error
- **WHEN** a configured skill root is missing or unreadable
- **THEN** the lookup lists the entries it could read and treats the missing
  root as empty rather than failing the lookup

#### Scenario: Library prompts are listed
- **WHEN** the library holds a prompt named "Run the gate" and the user types
  `/gate`
- **THEN** the popup lists that prompt, marked as a library prompt

#### Scenario: A library change reaches an open composer
- **WHEN** a prompt is added in Settings while a panel composer is open
- **THEN** the next slash lookup in that composer lists the new prompt

## ADDED Requirements

### Requirement: Inserting a library prompt expands its text

Inserting a library prompt's entry SHALL replace the partially typed slash
token with the prompt's full text, line breaks included, with its variables
expanded for the selected agent (see `prompt-library` - Prompt variables),
and leave the rest of the buffer untouched.

Expansion SHALL NOT block the window on I/O (resolving `branch` reads the
repository). The composer SHALL stay responsive while it resolves, and the
text SHALL be inserted at the token's position once it has. If the user edits
the token or dismisses the lookup before resolution completes, nothing SHALL
be inserted.

The expanded text SHALL be ordinary composer text: the user can edit it
before sending, and nothing is sent by the insertion itself.

This differs from "Inserting an entry completes text", which inserts a
command's or skill's token; a library prompt has no token the agent would
recognise, so its text is what is inserted.

#### Scenario: A prompt expands in place
- **WHEN** the buffer holds `note:` on one line and `/gat` at the start of
  the next, and the user inserts the library prompt "Run the gate" whose
  text is `make, then commit`
- **THEN** the buffer holds `note:` on the first line and
  `make, then commit` on the second

#### Scenario: Variables are expanded on insertion
- **WHEN** the user inserts a library prompt whose text is
  `Review {{folder.name}}` into the composer of an agent in `/src/knot`
- **THEN** the buffer holds `Review knot`

#### Scenario: A stale expansion is dropped
- **WHEN** the user inserts a library prompt and edits the slash token before
  its expansion has resolved
- **THEN** the expansion is discarded and the buffer keeps the user's edit

#### Scenario: Expansion does not send
- **WHEN** the user inserts a library prompt
- **THEN** nothing is sent to the agent until the user sends the composer
