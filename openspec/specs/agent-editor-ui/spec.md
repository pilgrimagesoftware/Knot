# agent-editor-ui Specification

## Purpose
Defines the new-agent and edit-agent dialog's own contract: the fields it
offers for an agent, what they default to, and what the dialog explains to
the user about them. The sidebar's `agent-list-ui` says when this dialog
opens; this capability says what it contains.

## Requirements

### Requirement: Activation control

The dialog SHALL offer a control for the agent's activation mode, and SHALL
name in words the mode the agent currently has - as `agent-lifecycle` names
them - rather than leaving it to be read off the control's position:
`passive` for a new agent, and the agent's own mode when editing an existing
one.

A hint SHALL sit next to that control saying what the two modes mean, in one
short sentence per mode. The difference is not inferable from the words
"active" and "passive" alone, and the cost of guessing wrong - an agent that
silently never starts, or a workspace that launches eight subprocesses -
falls on the user.

Submitting the dialog SHALL apply the chosen mode to the agent. Changing only
the activation mode SHALL NOT restart a running agent, per `agent-lifecycle`.

#### Scenario: A new agent offers passive

- **WHEN** the user opens the dialog to create an agent
- **THEN** the activation control shows `passive`, and the hint explains both
  modes

#### Scenario: Editing shows the agent's current mode

- **WHEN** the user opens the dialog on an existing `active` agent
- **THEN** the activation control shows `active`

#### Scenario: Changing the mode of a running agent does not disturb it

- **WHEN** the user edits a running agent, changes only its activation mode,
  and submits
- **THEN** the agent's mode is updated and the agent keeps running

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

### Requirement: Startup prompt control

The agent editor SHALL offer a Startup Prompt control for every agent whose
type is not `shell`, in both the new-agent and edit-agent forms. It SHALL
offer: None (the default for a new agent), each library prompt by name, and
Custom. Choosing Custom SHALL reveal a multi-line text field for the agent's
own text.

The control SHALL be absent for a `shell`-type agent, which has no coding
agent to prompt; switching an agent's type to `shell` SHALL clear its startup
prompt when the form is submitted.

When the agent's startup prompt references a prompt no longer in the library,
the control SHALL show it as a missing prompt rather than as None, so the
user can see why nothing is sent and choose a replacement. Leaving it
unchanged SHALL keep the reference.

The custom text field SHALL offer the same variable list as the Prompts
tab's editor and SHALL flag unknown variable names as warnings without
blocking submit (see `prompt-library` - Unknown variables and escaping).

A caption under the control SHALL say that the prompt is sent after the
agent registers, on each new conversation, and not when a conversation is
resumed.

Changing only the startup prompt SHALL NOT restart the agent: it takes effect
on the agent's next fresh session (see `agent-lifecycle` - Edit triggers
restart only for launch-affecting changes).

#### Scenario: New agent defaults to no startup prompt

- **WHEN** the user opens the editor to create an agent
- **THEN** the Startup Prompt control shows None

#### Scenario: Choosing Custom reveals the text field

- **WHEN** the user chooses Custom in the Startup Prompt control
- **THEN** a multi-line text field appears for the startup prompt's text

#### Scenario: An unknown variable in custom text is flagged

- **WHEN** the user types `{{brnach}}` into the custom startup-prompt field
- **THEN** a warning names `brnach` as unknown, and the form can still be
  submitted

#### Scenario: A shell agent has no startup prompt

- **WHEN** the editor is open for an agent of type `shell`
- **THEN** the Startup Prompt control is absent

#### Scenario: A dangling reference is shown as missing

- **WHEN** the editor opens for an agent whose startup prompt references a
  removed library prompt
- **THEN** the control shows a missing-prompt entry, not None

#### Scenario: Editing the startup prompt does not restart

- **WHEN** the user changes only an agent's startup prompt and saves
- **THEN** the agent keeps running without a restart
