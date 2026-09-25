# Spec Delta

## ADDED Requirements

### Requirement: Prompts tab

The Settings window SHALL have a Prompts tab listing every library prompt
(name and a truncated, single-line preview of its text) in library order,
with per-row edit and delete actions and an "Add Prompt…" action. Adding and
editing SHALL use one editor with a name field and a multi-line text field;
its save action SHALL be disabled while either field is blank.

The editor SHALL list the prompt variables (see `prompt-library` - Prompt
variables) with what each expands to, and choosing one SHALL insert it at the
text field's caret. It SHALL flag unknown variable names as warnings without
disabling save (see `prompt-library` - Unknown variables and escaping).

Deleting a prompt that no agent or bench entry references SHALL happen
immediately. Deleting one that is referenced SHALL first ask for
confirmation, and the confirmation SHALL say how many agents and bench
entries reference it, since each of them will launch without a startup
prompt afterwards.

#### Scenario: Empty library shows a message

- **WHEN** the library holds no prompts
- **THEN** the Prompts tab shows a message saying no prompts are defined
  instead of an empty list

#### Scenario: Adding a prompt

- **WHEN** the user clicks "Add Prompt…", enters a name and text, and saves
- **THEN** the prompt is added to the library and appears at the end of the
  list immediately

#### Scenario: Save is disabled for a blank field

- **WHEN** the prompt editor's name is filled in and its text is blank
- **THEN** the save action is disabled

#### Scenario: Inserting a variable from the list

- **WHEN** the user places the caret in the text field and chooses
  `folder.name` from the variable list
- **THEN** `{{folder.name}}` is inserted at the caret

#### Scenario: Canceling an edit discards changes

- **WHEN** the user opens the editor for a prompt, changes its text, and
  cancels
- **THEN** the prompt's stored name and text are unchanged

#### Scenario: Deleting a referenced prompt asks first

- **WHEN** the user deletes a prompt that one agent and one bench entry
  reference
- **THEN** a confirmation names one agent and one bench entry, and canceling
  it leaves the prompt in the library

#### Scenario: Deleting an unreferenced prompt does not ask

- **WHEN** the user deletes a prompt nothing references
- **THEN** it is removed immediately, with no confirmation

### Requirement: Bench tab

The Settings window SHALL have a Bench tab listing every bench entry (avatar,
name, folder's last path component, and its startup prompt's name or "Custom"
or nothing) with per-row edit and remove actions. Editing SHALL change the
entry's name and startup prompt only; the other template fields come from the
agent the entry was saved from and are changed by saving that agent to the
bench again.

Removing an entry SHALL ask for confirmation naming the entry, matching the
Swift app's bench dropdown.

The Swift app edits the bench only from the New Agent button's dropdown, and
there only removes entries. The Rust port keeps that dropdown (see
`agent-list-ui` - The New Agent button opens the bench) and adds this tab for
editing, which the dropdown has no room for.

#### Scenario: Empty bench shows how to fill it

- **WHEN** the bench holds no entries
- **THEN** the Bench tab shows a message saying an agent is added from its
  row menu's Save to Bench or Bench Agent

#### Scenario: Renaming a bench entry

- **WHEN** the user edits a bench entry's name and saves
- **THEN** the stored entry carries the new name and its other fields are
  unchanged

#### Scenario: Removing a bench entry asks first

- **WHEN** the user removes a bench entry and cancels the confirmation
- **THEN** the entry is still on the bench
