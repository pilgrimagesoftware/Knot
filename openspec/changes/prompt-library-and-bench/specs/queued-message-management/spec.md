# Spec Delta

## MODIFIED Requirements

### Requirement: A queued prompt names where it came from

A queued prompt that the system generated rather than the user SHALL be
identified as such by its row's accessible name and tooltip, so a prompt the
user never typed does not read as one they did. A prompt whose delivery
failed SHALL report the failure instead, since that is the state the user
has to act on.

A startup prompt queued behind the initialization turn (see
`agent-launch-command`) is one such prompt, and SHALL be named as a queued
startup prompt, distinct from both a queued user prompt and a queued inbox
check.

#### Scenario: A queued inbox nudge is labelled as one

- **WHEN** an automatic inbox nudge is waiting in an agent's queue
- **THEN** its status control's accessible name identifies it as a queued
  inbox check, distinct from the name used for a queued user prompt

#### Scenario: A failed nudge reports the failure

- **WHEN** an automatic inbox nudge in the queue has failed to deliver
- **THEN** its status control reports the failure rather than its origin

#### Scenario: A queued startup prompt is labelled as one

- **WHEN** an agent's startup prompt is waiting behind its initialization
  turn
- **THEN** its status control's accessible name identifies it as a queued
  startup prompt, distinct from a queued user prompt and a queued inbox check
