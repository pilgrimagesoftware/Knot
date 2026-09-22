# Spec Delta

## ADDED Requirements

### Requirement: Inbox nudges preserve interrupted session work

An automatic inbox nudge SHALL be queued behind the agent's active session and
SHALL NOT interrupt, replace, discard, or permanently suspend the currently
pending task. The nudge text SHALL tell the agent to check its inbox and, if
there is nothing to do, continue its previous work. After the nudge is
acknowledged or handled, the agent session SHALL continue the pending task
from its prior state.

#### Scenario: Nudge arrives while a task is paused

- **WHEN** an agent has pending work and receives an automatic "check your
  inbox" nudge
- **THEN** the nudge waits behind the pending work, and its text tells the
  agent to continue the previous work if there is nothing to do

#### Scenario: Nudge interrupts delivery of a task update

- **WHEN** an inbox nudge arrives while the agent session is delivering an
  update for its current task
- **THEN** the nudge is queued without interrupting the update, and the update
  state and current task resume afterward

#### Scenario: Ordinary user message remains independent

- **WHEN** a user sends a normal message rather than an automatic inbox nudge
- **THEN** the existing prompt and queue rules apply without continuation
  semantics being inferred
