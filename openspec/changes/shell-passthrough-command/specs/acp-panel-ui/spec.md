# Spec Delta

## MODIFIED Requirements

### Requirement: Streaming message rendering
The system SHALL render assistant text as it streams (text deltas appended
to the current message, not replaced), and SHALL visually distinguish user
messages, assistant messages, system/tool content, and shell-command results
the user ran from the prompt input.

#### Scenario: Rapid successive text deltas
- **WHEN** the agent emits several text deltas for the same message in quick
  succession
- **THEN** the panel reflects the latest accumulated text without visible
  flicker or reordering

#### Scenario: A shell result reads as neither message nor tool call
- **WHEN** the conversation holds a user message, an assistant message, a
  tool call, and a shell-command result
- **THEN** each is visually distinct from the others, so the reader can tell
  the user ran the command rather than the agent

### Requirement: Input area send control

The input area SHALL provide a send control that submits the pending message
(with any attached context) to the agent. The control SHALL be disabled while
the input is empty and while a permission request is pending. While a
response is in progress, activating the control SHALL enqueue the message for
delivery when the current turn ends; it SHALL NOT discard the message and
SHALL NOT be disabled on that account. While a response is in progress, the
input area SHALL also provide a stop control that interrupts the active ACP
turn for the selected session.

A pending message recognised as a shell command SHALL be exempt from these
rules: the control SHALL remain enabled for it while a permission request is
pending and while a response is in progress, and activating it SHALL run the
command rather than send or enqueue a message. The stop control SHALL keep
interrupting only the ACP turn; it SHALL NOT terminate a running shell
command.

#### Scenario: Send a message
- **WHEN** the user activates send with non-empty input and no response is in
  progress
- **THEN** the message and any attached context are submitted to the agent
  and the input area clears

#### Scenario: Send during a response enqueues
- **WHEN** the user activates send with non-empty input while a response is
  in progress
- **THEN** the message joins the prompt queue, the input area clears, and the
  message is delivered when the current turn ends

#### Scenario: A shell command sends during a response
- **WHEN** the user activates send on a shell command while a response is in
  progress
- **THEN** the command runs immediately, the input area clears, and the
  prompt queue is unchanged

#### Scenario: A shell command sends while permission is pending
- **WHEN** a permission request is pending and the input holds a shell
  command
- **THEN** the send control is enabled and activating it runs the command

#### Scenario: Stop an active turn
- **WHEN** the user activates stop while an ACP turn is in progress
- **THEN** the selected session receives a cancellation request and the
  control remains safe to activate again until the turn reaches a terminal
  state

#### Scenario: Stop leaves a running command alone
- **WHEN** the user activates stop while both an ACP turn and a shell command
  are running
- **THEN** the turn is cancelled and the shell command continues

#### Scenario: Cancellation completes
- **WHEN** the active turn is cancelled or finishes after a stop request
- **THEN** the stop control disappears, the input area returns to its normal
  state, and no later turn is cancelled

#### Scenario: Stop is scoped to one session
- **WHEN** the user stops work in one panel while another panel is active
- **THEN** only the selected panel's ACP turn is interrupted
