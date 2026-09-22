# Spec Delta

## ADDED Requirements

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
