# Spec Delta

## ADDED Requirements

### Requirement: Inbox nudges preserve interrupted session work

An automatic "check your inbox" nudge SHALL be handled as an auxiliary
message and SHALL NOT replace, discard, or permanently suspend the agent's
currently pending task. After the inbox nudge is acknowledged or handled, the
agent session SHALL continue the pending task from its prior state.

#### Scenario: Nudge arrives while a task is paused

- **WHEN** an agent has pending work and receives an automatic "check your
  inbox" nudge
- **THEN** the nudge is surfaced without replacing the pending work, and the
  session can continue that work afterward

#### Scenario: Nudge interrupts delivery of a task update

- **WHEN** an inbox nudge arrives while the agent session is delivering an
  update for its current task
- **THEN** the update state is retained and the current task resumes after the
  nudge is processed

#### Scenario: Ordinary user message remains independent

- **WHEN** a user sends a normal message rather than an automatic inbox nudge
- **THEN** the existing prompt and queue rules apply without continuation
  semantics being inferred
