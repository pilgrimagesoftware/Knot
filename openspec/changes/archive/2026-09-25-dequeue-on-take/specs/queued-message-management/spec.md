## ADDED Requirements

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

## REMOVED Requirements

### Requirement: A message being delivered cannot be edited

**Reason**: A message being delivered is no longer in the queue, so it has no
row to offer an edit control on.

**Migration**: None.
