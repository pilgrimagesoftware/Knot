# workspace-manager-ui Specification

## Purpose

Defines the Workspaces window - the list of workspaces it shows and the
dialogs it opens to create, rename and delete them - and how those dialogs are
operated by keyboard as well as by pointer. Distinct from `agent-list-ui`,
which covers the workspace window's agent sidebar, and from `settings-ui`,
which covers the settings window.

## Requirements

### Requirement: The workspace name dialog is operable by keyboard

The dialog that names a workspace - the one opened to create a new workspace
and the one opened to rename an existing one, which are the same dialog in two
modes - SHALL respond to Return and Escape as it responds to its two buttons.

Pressing Return SHALL do exactly what the dialog's confirm button does:
create the workspace, or rename the existing one, and close the dialog.
Pressing Escape SHALL do exactly what Cancel does: close the dialog and
discard whatever was typed, changing nothing.

Both keys SHALL work while the name field has focus, which is where focus
sits when the dialog opens. A dialog a user can begin on the keyboard and
cannot finish there is the gap this closes.

#### Scenario: Return creates a workspace

- **WHEN** the user opens the new-workspace dialog, types a name, and
  presses Return
- **THEN** a workspace with that name is created and the dialog closes,
  exactly as clicking Create would

#### Scenario: Return renames a workspace

- **WHEN** the user opens the dialog to rename an existing workspace, edits
  the name, and presses Return
- **THEN** that workspace is renamed and the dialog closes, exactly as
  clicking Save would

#### Scenario: Escape discards the dialog

- **WHEN** the user has typed a name and presses Escape
- **THEN** the dialog closes, no workspace is created or renamed, and the
  typed name is discarded

#### Scenario: Escape after an error still discards

- **WHEN** the dialog is showing a validation error and the user presses
  Escape
- **THEN** the dialog closes and the error is cleared, leaving nothing
  behind for the next time the dialog opens

### Requirement: An empty workspace name cannot be confirmed

A workspace name that is empty, or consists only of whitespace, SHALL NOT
create or rename a workspace by any route - Return, the confirm button, or
both.

The confirm button SHALL be disabled while the name is empty or
whitespace-only. Pressing Return in that state SHALL do nothing: the dialog
stays open with the name field still focused, and no error is raised by the
keypress. The disabled button is what tells the user why nothing happened;
an error message fired by a key the user pressed to make progress explains
less than a button that visibly cannot be pressed.

Leading and trailing whitespace SHALL be trimmed from a name before it is
stored, so a name is judged empty on its trimmed form.

#### Scenario: Return with an empty name does nothing

- **WHEN** the dialog is open with an empty name field and the user presses
  Return
- **THEN** the dialog stays open, no workspace is created, and no error
  message appears

#### Scenario: Return with a whitespace-only name does nothing

- **WHEN** the name field contains only spaces and the user presses Return
- **THEN** the dialog stays open and no workspace is created

#### Scenario: The confirm button reflects the name's validity

- **WHEN** the dialog opens for a new workspace, with an empty name
- **THEN** the Create button is disabled
- **WHEN** the user types a non-whitespace character
- **THEN** the Create button becomes enabled
- **WHEN** the user deletes it again
- **THEN** the Create button is disabled again

#### Scenario: Surrounding whitespace is trimmed

- **WHEN** the user types a name with leading and trailing spaces and
  presses Return
- **THEN** the workspace is created with the trimmed name
