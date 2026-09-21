# Spec Delta

## MODIFIED Requirements

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
