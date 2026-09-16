## Purpose

Renders an ACP agent session as a native chat-like panel — streaming
messages, tool calls, diffs, and permission prompts — as the default view
for ACP-managed agents, with an explicit toggle back to the raw terminal.

## ADDED Requirements

### Requirement: Panel is the default view for ACP-managed agents
When an agent's type has a registered ACP adapter and the agent is not
explicitly set to Terminal mode, the system SHALL show the panel view instead
of the terminal grid for that agent.

#### Scenario: Agent type has no ACP adapter
- **WHEN** an agent's type has no registered ACP adapter
- **THEN** the system SHALL show the terminal view for that agent and SHALL
  NOT offer the panel as a view mode

### Requirement: View-mode toggle
The system SHALL let the user switch a single agent between Panel and
Terminal view independently of other agents, and SHALL persist the last
chosen mode per agent. Switching modes SHALL NOT interrupt an in-flight ACP
turn or terminal process.

#### Scenario: Switch to Terminal mid-turn
- **WHEN** the user switches an agent from Panel to Terminal while a prompt
  turn is streaming
- **THEN** the turn continues running and its remaining updates are applied to
  panel state so switching back shows the complete conversation

### Requirement: Streaming message rendering
The system SHALL render assistant text as it streams (text deltas appended
to the current message, not replaced), and SHALL visually distinguish user
messages, assistant messages, and system/tool content.

#### Scenario: Rapid successive text deltas
- **WHEN** the agent emits several text deltas for the same message in quick
  succession
- **THEN** the panel reflects the latest accumulated text without visible
  flicker or reordering

### Requirement: Tool-call rendering
The system SHALL render each tool call as a distinct card showing its kind,
input summary, and result (or in-progress state) once received, and SHALL
render a file-edit tool call's diff as an added/removed line view rather than
raw text.

#### Scenario: Tool call fails
- **WHEN** a tool call's result reports an error
- **THEN** the card SHALL indicate failure and show the error content instead
  of a normal result

### Requirement: Permission prompts
The system SHALL render a pending permission request inline in the
conversation with its available options as actionable controls, SHALL block
sending further prompts for that session until the request is answered, and
SHALL send the user's choice back through the ACP client.

#### Scenario: User denies a permission request
- **WHEN** the user selects a deny option on a permission prompt
- **THEN** the system sends that decision to the agent and the prompt is
  replaced with its resolved state (not left pending)

### Requirement: Terminal remains available
The system SHALL NOT remove or degrade the existing terminal view. Any agent
— ACP-managed or not — SHALL be able to open a real terminal for that
agent's working directory, independent of ACP session state.

#### Scenario: Panel-mode agent opens a terminal
- **WHEN** the user opens a terminal for an agent currently in Panel mode
- **THEN** the system spawns a plain shell in that agent's working directory,
  independent of and without disturbing its ACP session
