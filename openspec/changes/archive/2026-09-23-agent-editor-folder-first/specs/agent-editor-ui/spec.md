# Spec Delta

## ADDED Requirements

### Requirement: The folder is asked for before the agent is described

The dialog SHALL present the agent's folder before the fields that describe
the agent - its type, its shell command, its persona, its activation mode, and
its registry metadata - and after the agent's name and avatar.

The folder is what the dialog cannot be submitted without: it must be chosen,
and it must name a directory that exists. A form SHALL NOT ask for its one
hard prerequisite after everything that is optional, because the user then
fills in what may be discarded before learning what is required.

The order of the remaining sections relative to each other SHALL NOT change,
and moving the folder SHALL NOT change what makes the dialog submittable.

#### Scenario: The folder is reached before the agent's type

- **WHEN** the user opens the dialog to create an agent and reads down the
  form
- **THEN** the folder is presented after the name and avatar and before the
  agent type, persona, activation mode and registry metadata

#### Scenario: Moving the folder does not change what is required

- **WHEN** the dialog is submitted with no folder chosen, or with a folder
  that is not an existing directory
- **THEN** it is refused for the same reason and with the same message as
  before, and the primary action stays disabled until a name and an existing
  folder are both present

### Requirement: Choosing a folder names an unnamed agent after it

When the user chooses a folder and the name field holds nothing - empty, or
only whitespace - the dialog SHALL put the folder's last path component in
the name field.

The value SHALL land in the field itself, where the user can see it and edit
it before submitting, rather than being supplied at submit time. An agent is
almost always named for the directory it works in, so this is the name the
user would have typed; showing it is what separates a default from a hidden
substitution.

A name the user has already supplied SHALL NOT be replaced. Choosing a second
folder, or correcting a mistaken one, SHALL leave a typed name exactly as it
is.

A folder with no usable last path component SHALL leave the name field
untouched rather than filling it with nothing.

This SHALL NOT weaken the requirement that an agent has a name: a dialog whose
name field is still empty SHALL still refuse to submit.

#### Scenario: A blank name takes the folder's name

- **WHEN** the user opens the dialog to create an agent, leaves the name
  field empty, and chooses the folder `/Users/dana/code/widget`
- **THEN** the name field shows `widget`

#### Scenario: A typed name survives choosing a folder

- **WHEN** the user types `Reviewer` in the name field and then chooses the
  folder `/Users/dana/code/widget`
- **THEN** the name field still shows `Reviewer`

#### Scenario: Correcting the folder does not rename the agent

- **WHEN** the user chooses `/Users/dana/code/widget` with a blank name, and
  then chooses `/Users/dana/code/gadget`
- **THEN** the name field shows `widget`, the name the first choice supplied,
  because it is no longer blank

#### Scenario: A whitespace-only name counts as blank

- **WHEN** the name field holds only spaces and the user chooses the folder
  `/Users/dana/code/widget`
- **THEN** the name field shows `widget`

#### Scenario: The filled name can be edited before submitting

- **WHEN** a folder has filled the name field and the user edits it to
  something else and submits
- **THEN** the agent is created with the edited name

#### Scenario: A name is still required

- **WHEN** the name field is empty and the user clears it after a folder
  filled it, then tries to submit
- **THEN** the dialog refuses and asks for a name, as it does for any other
  unnamed agent
