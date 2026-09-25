# Spec Delta

## ADDED Requirements

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
