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

The prompt SHALL identify the tool call by a human-readable name rather than
its opaque id when one is known: the title carried by the request if present,
else the title of the panel's known tool-call card for that id, else the
call's kind. If none of these is known, the raw id SHALL be shown rather than
a blank.

#### Scenario: User denies a permission request
- **WHEN** the user selects a deny option on a permission request
- **THEN** the system sends that decision to the agent and the prompt is
  replaced with its resolved state (not left pending)

#### Scenario: The prompt names the tool
- **WHEN** a permission request arrives for a tool call the panel knows by
  the title "Reading configuration file"
- **THEN** the prompt reads "Permission requested for Reading configuration
  file" (or equivalent), not the call's opaque id

#### Scenario: An unnamed call falls back to its id
- **WHEN** a permission request carries no title and no matching card is
  known
- **THEN** the prompt shows the call's id rather than an empty name

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

The call's status SHALL be expressed by a status icon, and the card's own
words about the call - the status word shown in the icon's tooltip - SHALL be
in the proportional family.

#### Scenario: A shell command reads as a command

- **WHEN** a tool call titled `git status --porcelain=v2` renders
- **THEN** its title is in the monospace family

#### Scenario: The status stays prose

- **WHEN** a completed tool call renders with the status word "Done" as the
  status icon's tooltip
- **THEN** that word is in the proportional family, beside a monospace title

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

### Requirement: Tool call status icon

The system SHALL render a tool call's status as an icon whose glyph and color
follow the call's state: an in-flight state (pending or running) SHALL use the
theme's info colour, a failed call the theme's danger colour, and a completed
call the same neutral muted treatment as the card's finished state. The word
for the status SHALL be carried by the icon's tooltip.

#### Scenario: A running call reads as running

- **WHEN** a tool call is `pending` or `in_progress`
- **THEN** its status icon renders in the theme's info colour and its tooltip
  says "Pending" or "Running…" as appropriate

#### Scenario: A failed call is marked

- **WHEN** a tool call reaches `failed`
- **THEN** its status icon renders in the theme's danger colour and its
  tooltip says "Failed"

#### Scenario: A completed call recedes

- **WHEN** a tool call reaches `completed`
- **THEN** its status icon renders in the neutral muted style and its tooltip
  says "Done"

#### Scenario: An unrecognized status keeps its text

- **WHEN** a tool call reports a status the panel does not recognize
- **THEN** the card shows the raw status string rather than a fabricated icon,
  matching the panel's existing pass-through behavior for unknown statuses

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
given response, the panel's virtualized list SHALL keep following the streamed
output for that response, auto-scrolling so the newest output stays visible.
Manually scrolling the history away from the tail SHALL disable following.
Disabling the toggle SHALL stop following even while the list is at the tail,
so following is only ever resumed by enabling the toggle or jumping to latest.

#### Scenario: Tracking follows streamed output

- **WHEN** the user enables the track toggle on an in-progress response and
  the panel is following the tail
- **THEN** the list stays at the end and the newest streamed output is
  visible as it arrives

#### Scenario: Manual scroll disables tracking

- **WHEN** the user enables the track toggle on an in-progress response and
  then scrolls the history away from the tail
- **THEN** the list stops auto-scrolling, so the content the user scrolled to
  stays put as further output streams

#### Scenario: Turning the toggle off stops following even at the tail

- **WHEN** the user disables the track toggle on an in-progress response
- **THEN** the list stops auto-scrolling and stays where it is as further
  output streams, even if it is at the tail, rather than snapping back to the
  end

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
the agent does not support concurrent input. While a response is in progress,
the input area SHALL also provide a stop control that interrupts the active
ACP turn for the selected session.

#### Scenario: Send a message
- **WHEN** the user activates send with non-empty input
- **THEN** the message and any attached context are submitted to the
  agent and the input area clears

#### Scenario: Stop an active turn
- **WHEN** the user activates stop while an ACP turn is in progress
- **THEN** the selected session receives a cancellation request and the
  control remains safe to activate again until the turn reaches a terminal
  state

#### Scenario: Cancellation completes
- **WHEN** the active turn is cancelled or finishes after a stop request
- **THEN** the stop control disappears, the input area returns to its normal
  state, and no later turn is cancelled

#### Scenario: Stop is scoped to one session
- **WHEN** the user stops work in one panel while another panel is active
- **THEN** only the selected panel's ACP turn is interrupted

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

### Requirement: A finished tool call collapses to its header

A tool call that has completed successfully SHALL render collapsed: its
header - icon, title and status - stays visible and its output is hidden.

A tool call that is still running SHALL stay expanded, since its output is
what the user is waiting on.

A tool call that failed SHALL stay expanded. Failure output is the reason the
user is reading the conversation at all, and hiding it behind a control makes
the one card that matters the one card they have to open.

Collapsing SHALL NOT discard anything: the content is hidden, not dropped,
and opening the card again shows exactly what was there.

#### Scenario: A call collapses when it succeeds

- **WHEN** a tool call the panel is showing expanded reaches `completed`
- **THEN** its output is hidden and its header remains, naming the tool and
  its status

#### Scenario: A running call stays open

- **WHEN** a tool call is `pending` or `in_progress`
- **THEN** it renders expanded

#### Scenario: A failed call stays open

- **WHEN** a tool call reaches `failed`
- **THEN** it renders expanded, with its output visible

#### Scenario: Reopening shows the same content

- **WHEN** the user opens a collapsed call
- **THEN** the output shown is the content that call finished with

### Requirement: Every tool call can be opened and closed

Each tool call card SHALL carry a control that toggles it between collapsed
and expanded, and that shows which of the two it currently is.

The control SHALL be present whatever the call's status - a running call can
be collapsed, a failed one can be closed - so the automatic behaviour above
is a default rather than a rule the user cannot escape.

#### Scenario: Closing a call the panel opened

- **WHEN** the user activates the control on an expanded call
- **THEN** that call collapses to its header

#### Scenario: Opening a call the panel closed

- **WHEN** the user activates the control on a collapsed call
- **THEN** that call expands and its output is shown

#### Scenario: A running call can be closed

- **WHEN** the user activates the control on an `in_progress` call
- **THEN** it collapses, and goes on streaming its output out of sight

### Requirement: The user's choice outlives the automatic one

Once the user has opened or closed a particular tool call, that choice SHALL
hold for that call for the rest of the session, including across the call
finishing. The automatic collapse SHALL apply only to a call the user has not
touched.

#### Scenario: A call opened while running stays open when it finishes

- **WHEN** the user opens a running call and it then reaches `completed`
- **THEN** it stays expanded

#### Scenario: A call closed while running stays closed when it finishes

- **WHEN** the user closes a running call and it then reaches `completed`
- **THEN** it stays collapsed

#### Scenario: An untouched call follows the default

- **WHEN** a call the user has never toggled reaches `completed`
- **THEN** it collapses

### Requirement: Collapse state is per call and not persisted

The open/closed state SHALL be tracked per tool call, so opening one leaves
the others as they were, and SHALL NOT be persisted: a reloaded conversation
starts from the automatic behaviour again.

#### Scenario: Opening one call leaves the others alone

- **WHEN** the user opens one of several collapsed calls
- **THEN** only that one expands

#### Scenario: A reloaded conversation starts fresh

- **WHEN** a session is restarted or its conversation reloaded
- **THEN** every finished call renders collapsed again, whatever the user had
  opened before

### Requirement: Collapsing does not move what the user is reading

A call collapsing on completion SHALL NOT scroll the conversation away from
what the user is looking at. While auto-scroll is following a response, the
panel SHALL stay at the end of the conversation; while it is not, the content
the user is reading SHALL stay where it is.

#### Scenario: A call finishes while the user reads earlier output

- **WHEN** the user has scrolled back to read earlier output and a call
  further down completes and collapses
- **THEN** what the user is reading stays in place

#### Scenario: A call finishes while auto-scroll is following

- **WHEN** auto-scroll is following a streaming response and a call above it
  collapses
- **THEN** the panel stays at the end of the conversation

### Requirement: Conversation rendering is virtualized

The conversation in the panel SHALL render through a virtualized list
scroller that lays out, measures and paints only the rows near the viewport
(plus a small overdraw margin), never the whole history. The per-frame work
and the retained layout state SHALL therefore be bounded by how much of the
conversation fits on screen, not by the conversation's total length, so a
long session does not get measurably more expensive the longer it runs.

Messages that have scrolled out of the viewport SHALL NOT be destroyed; they
are skipped for that frame's layout and painting only, so their full content
is still reachable by scrolling. Materializing a row later SHALL yield the
same content and state it would have had if the whole conversation had been
built eagerly.

#### Scenario: A long conversation stays cheap to render

- **WHEN** a conversation contains far more messages than can fit on screen at
  once
- **THEN** the panel lays out and paints only the messages near the viewport,
  and remains fluent as the user scrolls, regardless of how long the
  conversation is

#### Scenario: A previously skipped message still has its content

- **WHEN** the user scrolls to a message that is not currently materialized
- **THEN** that message renders with the full content and state it would have
  had if the whole conversation had been built eagerly

### Requirement: Streaming keeps the visible message anchored

As a message streams and grows, the row it belongs to SHALL be re-measured and
updated in place, without re-laying-out or tearing down the rest of the
conversation, so the content the user is currently reading does not jump. When
the user is following the tail, the panel SHALL stay at the end of the
conversation so newly streamed output remains visible as it arrives.

#### Scenario: Streaming output while reading earlier history

- **WHEN** the user has scrolled away from the tail and a visible message
  gains more streamed text
- **THEN** the growing row is re-measured in place and what the user is
  reading stays where it is

#### Scenario: Streaming output while following the tail

- **WHEN** the panel is following the tail and a new message begins to stream
- **THEN** the panel stays at the end of the conversation and the new content
  appears as it streams

### Requirement: Per-message and jump controls reach any message

The panel's scroll controls (scroll to the originating user message, scroll to
top, and jump to latest) SHALL operate on the virtualized list and SHALL reach
any message even if it is not currently materialized.

#### Scenario: Scroll to an unmaterialized user message

- **WHEN** the user activates "scroll to user input" for a response whose user
  message lies far above the current viewport
- **THEN** the panel scrolls that user message into view

#### Scenario: Jump to latest resumes tail following

- **WHEN** the user activates the jump-to-latest control after scrolling the
  history away from the tail
- **THEN** the panel scrolls to the newest message and resumes following
  streamed output

#### Scenario: Scroll to top from anywhere

- **WHEN** the user activates the scroll-to-top control
- **THEN** the panel shows the earliest message regardless of how far it is
  from the viewport

### Requirement: Collapsed tool call header is a single line

A collapsed tool call SHALL render its header on a single line: the title
SHALL truncate with an ellipsis when it does not fit rather than wrap, and
the status indicator SHALL remain visible on that same line. An expanded tool
call SHALL NOT be affected.

#### Scenario: A long title ellipsizes when collapsed

- **WHEN** a collapsed tool call's title is longer than the header's width
- **THEN** the header is one line tall and the title ends in an ellipsis

#### Scenario: The indicator stays on the line

- **WHEN** a collapsed tool call's title is ellipsized
- **THEN** the status indicator remains visible at the row's end rather than
  being pushed off or wrapped

#### Scenario: Expanded cards are unchanged

- **WHEN** the user expands a tool call whose title is long
- **THEN** the header behaves as it does today and the title is free to wrap

### Requirement: Input area context indicator
The input area SHALL display a radial indicator in its bottom bar when the ACP
session reports a positive context-window size. Its fill SHALL represent used
context tokens as a fraction of the reported total; its remainder SHALL
represent available context. Its tooltip SHALL state the used and total token
counts. The indicator SHALL be absent until the agent reports usable values.

#### Scenario: A usage update fills the indicator
- **WHEN** an ACP session reports 53,000 used tokens from a 200,000-token
  context window
- **THEN** the radial indicator is filled to 26.5 percent and its tooltip
  identifies both values

#### Scenario: No usage update is available
- **WHEN** the ACP agent does not report context-window usage
- **THEN** the bottom bar does not show a context indicator
