# queued-message-management Specification

## Purpose
Lets users remove individual prompts that are waiting in an agent's queue before those prompts are submitted.

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

### Requirement: A message being delivered cannot be edited

The system SHALL NOT offer an edit action for a message whose delivery has started, so that
editing never alters a prompt the agent has already been given.

#### Scenario: No edit control on a message in flight
- **WHEN** a queued message has been handed to the agent and its turn is running
- **THEN** its row exposes no edit control

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
