## ADDED Requirements

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
