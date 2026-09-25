# prompt-library Specification

## Purpose
Defines the prompt library - named prompts the user writes once and reuses as
agents' startup prompts and from the panel's `/` lookup - the startup prompt
an agent or bench entry carries, by library reference or as its own text,
and the variables prompt text can use to adapt to the agent it is sent to.

## Requirements

### Requirement: Prompt model

A library prompt SHALL have an id, a name and a text. The name SHALL be
non-empty after trimming surrounding whitespace, and the text SHALL be
non-empty after trimming. Names SHALL NOT be required to be unique; the id is
what identifies a prompt.

The text MAY span several lines, and SHALL be stored exactly as entered.

The Swift app has no prompt library; this capability is new to the Rust port.

#### Scenario: A blank name is refused

- **WHEN** a prompt is added whose name is only whitespace
- **THEN** the add is refused and the library is unchanged

#### Scenario: Multi-line text is stored as entered

- **WHEN** a prompt is added whose text holds three lines
- **THEN** reloading the library yields the same three lines, unchanged

### Requirement: Library operations

The library SHALL support adding a prompt, updating a prompt's name and text
by id, and removing a prompt by id. The library SHALL keep prompts in the
order they were added. Each operation SHALL persist the library document
before it reports success.

Removing a prompt SHALL NOT rewrite any agent or bench entry that references
it; such a reference then resolves as described in "A reference to a removed
prompt".

#### Scenario: Updating a prompt changes every agent that references it

- **WHEN** two agents reference library prompt P as their startup prompt and
  P's text is updated
- **THEN** each agent's next fresh session is sent P's new text

#### Scenario: Removing a prompt leaves referencing agents untouched

- **WHEN** a prompt referenced by an agent is removed
- **THEN** the agents document is not rewritten by the removal

### Requirement: Startup prompt forms

An agent's or bench entry's startup prompt SHALL be one of: none, a reference
to a library prompt by id, or custom text held on the agent or entry itself.
Custom text SHALL follow the same non-empty rule as a library prompt's text; a
custom startup prompt whose text is blank SHALL be stored as none.

Resolving a startup prompt SHALL yield the text to send: nothing for none, the
referenced prompt's current text for a reference, and the held text for custom
text - in each case with its variables expanded for the agent it is sent to
(see "Prompt variables").

#### Scenario: A reference resolves to the prompt's current text

- **WHEN** an agent's startup prompt references library prompt P
- **THEN** resolving it yields P's text as it stands at resolution time

#### Scenario: Blank custom text is none

- **WHEN** the user saves an agent with custom startup-prompt text that is
  only whitespace
- **THEN** the agent is stored with no startup prompt

### Requirement: A reference to a removed prompt

A startup prompt that references an id no longer in the library SHALL resolve
to nothing: no startup prompt is sent, and the launch otherwise proceeds
normally. The reference SHALL be kept, not cleared, so that anything showing
the agent's startup prompt can report it as missing.

#### Scenario: Launch with a dangling reference

- **WHEN** an agent whose startup prompt references a removed prompt starts a
  fresh session
- **THEN** the initialization prompt is sent, no startup prompt follows it,
  and no error is shown in the conversation

### Requirement: Prompt variables

Prompt text - a library prompt's and a custom startup prompt's alike - SHALL
support the following variables, each written as its name between `{{` and
`}}`, with optional whitespace inside the braces (`{{ folder }}` is
`{{folder}}`):

| Variable | Expands to |
|---|---|
| `agent.name` | the agent's name |
| `agent.id` | the agent's knot agent ID |
| `agent.type` | the agent's type, e.g. `claude` |
| `folder` | the agent's folder, as an absolute path |
| `folder.name` | the last path component of the agent's folder |
| `workspace` | the name of the workspace the agent belongs to; empty if none |
| `branch` | the current branch of the agent's folder; the short commit hash when HEAD is detached; empty when the folder is not in a git repository |
| `date` | today's date in the local time zone, as `YYYY-MM-DD` |

The set is closed: these are the only names that expand. Names SHALL be
matched case-sensitively.

Expansion SHALL happen when the text is used, never when it is stored: the
library and every startup prompt keep the variables as written, so one
prompt serves every agent, and `branch` and `date` are current each time.

Values SHALL be inserted literally; a value that itself contains `{{` SHALL
NOT be expanded again.

The Swift app has no prompt variables.

#### Scenario: Variables expand for the receiving agent

- **WHEN** the prompt `Work in {{folder.name}} on {{branch}}` is sent to an
  agent in `/src/knot` whose folder is on branch `471-restart`
- **THEN** the text sent is `Work in knot on 471-restart`

#### Scenario: One library prompt adapts per agent

- **WHEN** two agents in different folders both reference a library prompt
  that uses `{{folder}}`
- **THEN** each is sent its own folder's path

#### Scenario: Whitespace inside the braces is allowed

- **WHEN** the text holds `{{ agent.name }}` and the agent is named "Knot 3"
- **THEN** it expands to `Knot 3`

#### Scenario: Branch outside a repository is empty

- **WHEN** the text uses `{{branch}}` and the agent's folder is not in a git
  repository
- **THEN** `{{branch}}` expands to the empty string

#### Scenario: A value is not expanded twice

- **WHEN** an agent is named `{{date}}` and the text is `{{agent.name}}`
- **THEN** the expanded text is the literal `{{date}}`

### Requirement: Unknown variables and escaping

A `{{...}}` whose name is not one of the prompt variables SHALL be left
exactly as typed, braces included, and SHALL NOT prevent the prompt from
being sent or inserted. Anything that edits prompt text - the Prompts tab's
editor and the agent editor's custom startup-prompt field - SHALL flag each
unknown name as a warning, without blocking save, so a misspelling is caught
before it reaches an agent.

A backslash immediately before `{{` SHALL write a literal `{{` and suppress
expansion of what follows it; the backslash itself SHALL NOT appear in the
expanded text. A `{{` with no closing `}}` SHALL be left as typed.

#### Scenario: An unknown name is left as typed

- **WHEN** the text holds `{{issue}}`
- **THEN** the expanded text holds `{{issue}}`, unchanged

#### Scenario: A misspelling is flagged but saveable

- **WHEN** the user types `{{foldr}}` into the prompt editor
- **THEN** the editor shows a warning naming `foldr` as unknown, and save
  remains enabled

#### Scenario: An escaped variable is written literally

- **WHEN** the text holds `\{{folder}}`
- **THEN** the expanded text holds `{{folder}}`, with no backslash and no
  path

#### Scenario: An unclosed brace is left alone

- **WHEN** the text holds `use {{ carefully`
- **THEN** the expanded text is `use {{ carefully`
