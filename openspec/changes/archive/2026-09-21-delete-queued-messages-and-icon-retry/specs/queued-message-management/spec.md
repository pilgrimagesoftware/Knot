# Spec Delta

## Purpose

Lets users remove individual prompts that are waiting in an agent's queue before those prompts are submitted.

## ADDED Requirements

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
