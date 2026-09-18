# acp-panel-ui Specification

## Purpose
Renders an ACP agent session as a native chat-like panel — streaming
messages, tool calls, diffs, and permission prompts — as the default view
for ACP-managed agents, with an explicit toggle back to the raw terminal.

## Requirements

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

### Requirement: Tool call icon
Each tool call rendered in the message list SHALL display an icon
identifying the tool, positioned before the tool call's name/summary text.
An unrecognized tool name SHALL fall back to a generic tool icon rather
than omitting the icon.

#### Scenario: Known tool renders with its icon
- **WHEN** an agent response includes a call to a recognized tool (e.g. a
  file read or a shell command)
- **THEN** the panel renders that tool call with the icon mapped to that
  tool, immediately preceding its label

#### Scenario: Unknown tool falls back to generic icon
- **WHEN** an agent response includes a call to a tool the panel has no
  specific icon for
- **THEN** the panel renders that tool call with a generic fallback icon

### Requirement: A tool call's header separates content from chrome by font

A tool call card's title - the command it ran, the path it read, whatever the
agent named the call - SHALL render in the theme's monospace family. It is a
command, a path or an identifier, and the project renders those monospace
everywhere else.

The call's status text SHALL render in the proportional family. It is the
panel's own words about the call, not anything the agent produced, and it
SHALL NOT be swept into monospace along with the title.

#### Scenario: A shell command reads as a command

- **WHEN** a tool call titled `git status --porcelain=v2` renders
- **THEN** its title is in the monospace family

#### Scenario: The status stays prose

- **WHEN** a completed tool call renders with the status text "Done"
- **THEN** that text is in the proportional family, beside a monospace title

### Requirement: A tool call's outline says what state it is in

A tool call card's outline SHALL be coloured by its status:

- **failed**: the theme's danger colour.
- **in progress** (pending or running): the theme's info colour.
- **completed**: the same neutral border every other card in the panel uses.

A completed call SHALL NOT be given a colour of its own. Success is the
common case, and a conversation whose every finished call is outlined in
green is one where nothing stands out - which defeats the outline's only
purpose, marking the two states worth looking at.

Colours SHALL come from the theme rather than fixed values, so the card
follows a theme change like the rest of the panel.

#### Scenario: A failed call is marked

- **WHEN** a tool call reaches `failed`
- **THEN** its outline is the theme's danger colour

#### Scenario: A running call is marked

- **WHEN** a tool call is `pending` or `in_progress`
- **THEN** its outline is the theme's info colour

#### Scenario: A completed call recedes

- **WHEN** a tool call reaches `completed`
- **THEN** its outline is the panel's neutral card border, the same as an
  untouched card

#### Scenario: A call's outline follows it through its lifecycle

- **WHEN** a call starts, runs and then completes
- **THEN** its outline is the info colour while it runs and the neutral
  border once it is done, without the card being rebuilt around it

### Requirement: Response action bar
Each completed agent response SHALL display an action bar with: copy
response, scroll to the user message that produced this response, and
scroll to the top of the conversation history.

#### Scenario: Copy response
- **WHEN** the user activates "copy response" on an agent response
- **THEN** the full text of that response is placed on the system
  clipboard

#### Scenario: Scroll to originating user message
- **WHEN** the user activates "scroll to user input" on an agent response
- **THEN** the panel scrolls the history so the user message that
  prompted this response is visible

#### Scenario: Scroll to top
- **WHEN** the user activates "scroll to top" on an agent response
- **THEN** the panel scrolls the history to its earliest message

### Requirement: Track response toggle
The response action bar SHALL include a track toggle. While enabled for a
given response, the panel SHALL auto-scroll to keep newly streamed output
for that response visible. Manually scrolling the history away from the
bottom SHALL disable tracking.

#### Scenario: Tracking follows streamed output
- **WHEN** the user enables the track toggle on an in-progress response
- **THEN** the panel auto-scrolls to follow new content as it streams in

#### Scenario: Manual scroll disables tracking
- **WHEN** tracking is enabled and the user scrolls the history away from
  the bottom
- **THEN** tracking is disabled and auto-scroll stops

### Requirement: Input area context attachment
The input area SHALL provide an add-context control that lets the user
attach files or images to the next message.

#### Scenario: Attach a file
- **WHEN** the user activates the add-context control and selects a file
- **THEN** the file is attached to the pending message and shown in the
  input area before send

### Requirement: Input area permission mode selector
The input area SHALL provide a selector for the agent's permission mode,
applied to the next message and subsequent turns until changed.

#### Scenario: Change permission mode
- **WHEN** the user selects a different permission mode from the selector
- **THEN** subsequent agent turns run under the newly selected permission
  mode

### Requirement: Input area model selector
The input area SHALL provide a selector for which model the agent uses,
applied starting with the next message.

#### Scenario: Change model
- **WHEN** the user selects a different model from the selector
- **THEN** the next message is sent using the newly selected model

### Requirement: Input area effort selector
The input area SHALL provide a selector for the agent's reasoning effort
level, applied starting with the next message.

#### Scenario: Change effort level
- **WHEN** the user selects a different effort level from the selector
- **THEN** the next message is sent using the newly selected effort level

### Requirement: Input area send control
The input area SHALL provide a send control that submits the pending
message (with any attached context) to the agent. The control SHALL be
disabled while the input is empty and while a response is in progress if
the agent does not support concurrent input.

#### Scenario: Send a message
- **WHEN** the user activates send with non-empty input
- **THEN** the message and any attached context are submitted to the
  agent and the input area clears

### Requirement: Input area expand and collapse
The input area SHALL provide an expand control that grows the input into
a larger multi-line editor, and collapses it back to its default size
when activated again.

#### Scenario: Expand input area
- **WHEN** the user activates the expand control on the default-size
  input area
- **THEN** the input area grows to a larger multi-line editing size

#### Scenario: Collapse input area
- **WHEN** the user activates the expand control on the expanded input
  area
- **THEN** the input area returns to its default size
