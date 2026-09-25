# Spec Delta

## ADDED Requirements

### Requirement: Startup prompt follows the initialization prompt

On a fresh session for an agent with a startup prompt that resolves to text
(see `prompt-library`), with its variables expanded for that agent at the
moment the session starts, the system SHALL send that text as a separate user
turn after the initialization prompt's turn. The initialization prompt - the
knot instructions, persona and registration request - SHALL be unchanged by
the presence of a startup prompt, and the startup prompt SHALL NOT be folded
into it.

The startup prompt SHALL NOT be sent when a session is resumed or loaded,
when the user re-sends registration with Register Agent, or when the
initialization prompt is not sent at all.

On the ACP path the startup prompt SHALL enter the agent's prompt queue
behind the initialization turn, so it is delivered when that turn ends and
can be edited or deleted before then like any queued prompt (see
`queued-message-management`).

The expanded text SHALL be sent with its line breaks unchanged.

The startup prompt is delivered only on the ACP path. Every agent that runs
in a terminal is a `shell`-type agent - a non-shell agent always launches
through its ACP adapter, with no terminal fallback - and a `shell`-type agent
carries no startup prompt (see `agent-editor-ui` - Startup prompt control).

The Swift app sends only the registration prompt; the startup prompt is new
to the Rust port.

#### Scenario: Fresh ACP session sends both turns in order

- **WHEN** an ACP agent with a startup prompt starts a fresh session
- **THEN** the initialization prompt is sent first, and the startup prompt is
  sent once that turn has ended, as its own user message

#### Scenario: The startup prompt is expanded for the agent

- **WHEN** an agent named "Knot 3" whose startup prompt is `I am
  {{agent.name}}` starts a fresh session
- **THEN** the startup prompt sent is `I am Knot 3`

#### Scenario: Resumed session sends neither

- **WHEN** an agent with a startup prompt resumes a prior conversation
- **THEN** neither the initialization prompt nor the startup prompt is sent

#### Scenario: Restart with New Conversation sends it again

- **WHEN** the user restarts an agent with a startup prompt using Restart
  with New Conversation
- **THEN** the new session receives the initialization prompt and then the
  startup prompt

#### Scenario: Register Agent does not resend it

- **WHEN** the user selects Register Agent on an agent with a startup prompt
- **THEN** only the registration prompt is sent

#### Scenario: Line breaks are kept

- **WHEN** an agent's startup prompt has two lines
- **THEN** the turn it is sent as holds both lines, the break between them
  intact
