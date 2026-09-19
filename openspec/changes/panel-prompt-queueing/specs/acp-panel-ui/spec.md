# Spec Delta

## MODIFIED Requirements

### Requirement: Input area send control
The input area SHALL provide a send control that submits the pending message
(with any attached context) to the agent. The control SHALL be disabled while
the input is empty and while a permission request is pending. While a
response is in progress, activating the control SHALL enqueue the message for
delivery when the current turn ends; it SHALL NOT discard the message and
SHALL NOT be disabled on that account.

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

## ADDED Requirements

### Requirement: Prompt queue
The system SHALL queue prompts delivered during a response and deliver them to
the agent in the order they were enqueued, one turn at a time, until the queue
empties. A queued prompt SHALL be visible in the conversation, clearly marked
as waiting until it is delivered, and SHALL read as a normal prompt once
delivered. Delivery SHALL resume on each turn end; a prompt whose delivery
fails SHALL be reported under that prompt, and the queue SHALL continue with
the remaining prompts.

#### Scenario: Prompts are delivered in order
- **WHEN** the user enqueues "first" and then "second" while a response is in
  progress
- **THEN** "first" is delivered when the current turn ends and "second" is
  delivered when the turn "first" starts ends in turn

#### Scenario: A queued prompt is marked until delivered
- **WHEN** a prompt is enqueued during a response
- **THEN** the conversation shows it as waiting, and once its turn starts the
  same message reads as a delivered prompt

#### Scenario: A failed queued delivery continues the queue
- **WHEN** a queued prompt's delivery fails
- **THEN** the failure is reported under that prompt, the turn ends, and the
  next queued prompt is delivered

#### Scenario: A clean queue delivers immediately
- **WHEN** the user sends while no prompt is queued and no response is in
  progress
- **THEN** the prompt is delivered immediately and no queue entry is created

### Requirement: Permission gate is independent of the queue
Queued delivery SHALL NOT bypass a pending permission request: a prompt must
not be delivered while a permission request is awaiting an answer, whether or
not it was enqueued, and the send control remains disabled while a permission
request is pending.

#### Scenario: A permission request holds the queue
- **WHEN** a permission request is pending and the queue holds prompts
- **THEN** no queued prompt is delivered until the permission request is
  answered, after which delivery resumes