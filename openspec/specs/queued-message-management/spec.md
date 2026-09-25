# queued-message-management Specification

## Purpose
Lets users remove individual prompts that are waiting in an agent's queue before those prompts are submitted, and governs how a queued prompt is delivered and identified - including one the system generated rather than the user.

## Requirements

### Requirement: Queued messages can be deleted

The system SHALL provide a delete action for each queued message. Activating it SHALL remove only that message from the pending queue and SHALL leave the active turn and other queued messages unchanged.

#### Scenario: Delete one queued message
- **WHEN** the user activates delete on a queued message
- **THEN** that message disappears from the queue and the remaining messages keep their order

### Requirement: Deletion does not affect active work

Deleting a queued message SHALL NOT cancel, alter, or retry a turn that has already started.

#### Scenario: Delete while a turn runs
- **WHEN** the user deletes a queued message while another response is streaming
- **THEN** the streaming response continues and only the queued message is removed

### Requirement: Delete action is accessible

The delete control SHALL be an icon button with an accessible label or tooltip that identifies its action without relying on visible text in the row.

#### Scenario: Identify delete control
- **WHEN** a queued message is displayed
- **THEN** its delete icon exposes a localized accessible name and tooltip

### Requirement: Queued messages can be edited

The system SHALL provide an edit action for each queued message that has not started delivery.
Activating it SHALL remove that message from the queue and place its text in the panel composer,
leaving the active turn and every other queued message unchanged. The removed message SHALL NOT
hold its place in the queue; sending the edited text SHALL enqueue it as a new message at the
back of the queue.

#### Scenario: Edit one queued message
- **WHEN** the user activates edit on a queued message
- **THEN** that message disappears from the queue, its text appears in the composer ready to
  change, and the remaining messages keep their order

#### Scenario: An edited message is re-queued at the back
- **WHEN** the user activates edit on the first of three queued messages and sends the edited
  text while a response is still in progress
- **THEN** the edited message is delivered after the two that were behind it

#### Scenario: Edit while a turn runs
- **WHEN** the user edits a queued message while another response is streaming
- **THEN** the streaming response continues and only the edited message leaves the queue

### Requirement: Typed composer text is not discarded silently

If the composer already holds text the user has typed, the system SHALL ask for confirmation
before replacing it with a queued message's text. Declining SHALL leave both the composer and
the queue unchanged.

#### Scenario: Replace an empty composer
- **WHEN** the user activates edit while the composer is empty
- **THEN** the message's text is placed in the composer without a prompt

#### Scenario: Decline to replace typed text
- **WHEN** the user activates edit while the composer holds typed text and declines the
  confirmation
- **THEN** the composer keeps what was typed and the message stays in the queue

### Requirement: Edit action is accessible

The edit control SHALL be an icon button with an accessible label and tooltip that identify its
action without relying on visible text in the row.

#### Scenario: Identify the edit control
- **WHEN** a queued message that has not started delivery is displayed
- **THEN** its edit icon exposes a localized accessible name and tooltip

### Requirement: Queued prompts drain for every agent

A prompt waiting in an agent's queue SHALL be delivered once that agent's
session can take it, whether or not the agent is the one currently selected.
Selecting a different agent SHALL NOT delay delivery of a prompt already
queued for another.

This matters because an automatic inbox nudge is queued for agents the user
is not looking at (see `mcp-messaging`'s "Inbox nudges preserve interrupted
session work"), and a queue drained only for the selected agent would hold
such a nudge until the user happened to click that agent.

#### Scenario: A queued prompt behind a background agent's turn

- **WHEN** a prompt is queued for an agent that is not selected, and that
  agent's turn ends
- **THEN** the queued prompt is delivered without the user selecting that
  agent

#### Scenario: Selecting another agent does not stall a queue

- **WHEN** the user selects a different agent while a prompt is queued
- **THEN** the queued prompt is still delivered when its own agent's session
  is free

### Requirement: A queued prompt names where it came from

A queued prompt that the system generated rather than the user SHALL be
identified as such by its row's accessible name and tooltip, so a prompt the
user never typed does not read as one they did. A prompt whose delivery
failed SHALL report the failure instead, since that is the state the user
has to act on.

#### Scenario: A queued inbox nudge is labelled as one

- **WHEN** an automatic inbox nudge is waiting in an agent's queue
- **THEN** its status control's accessible name identifies it as a queued
  inbox check, distinct from the name used for a queued user prompt

#### Scenario: A failed nudge reports the failure

- **WHEN** an automatic inbox nudge in the queue has failed to deliver
- **THEN** its status control reports the failure rather than its origin

### Requirement: A queued message leaves the queue when delivery starts

The system SHALL remove a queued message from the queue at the moment it is
handed to the agent, not when the agent's turn ends. If the delivery fails,
the message SHALL return to the head of the queue, marked failed, keeping its
text and identity, so the user can retry or delete it.

#### Scenario: The row disappears when the prompt is sent
- **WHEN** the agent's turn ends and the message at the head of the queue is
  handed to it
- **THEN** that message's row disappears from the queue while the new turn
  runs

#### Scenario: A failed delivery returns to the queue
- **WHEN** a message taken from the queue fails to deliver
- **THEN** it reappears at the head of the queue marked failed
