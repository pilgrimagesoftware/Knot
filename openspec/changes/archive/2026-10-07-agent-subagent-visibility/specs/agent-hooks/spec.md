# Spec Delta

## ADDED Requirements

### Requirement: Subagent lifecycle hook events

The status route SHALL accept hook events reporting a subagent's dispatch and
a subagent's completion, in addition to the activity statuses it already
accepts. Such an event SHALL be identified by its `hook` field rather than by
its `status` field, so that reporting a subagent never moves the agent's
activity status as a side effect.

A dispatch event's payload SHALL carry an identifier for the subagent, unique
within the posting agent's session, and the task the subagent was given. It MAY
carry the kind of subagent dispatched; where it does not, the subagent's kind
is unstated.

A completion event's payload SHALL carry the same identifier, and SHALL say
whether the subagent succeeded or failed. A failure MAY carry a reason.

A subagent event whose payload lacks the identifier, or lacks the task on a
dispatch, SHALL yield a 400 and change no state. An event naming an identifier
Knot holds no dispatch for SHALL be accepted and ignored rather than rejected:
Knot may have started after the dispatch, and a 400 would make the agent's hook
report an error for something the agent did correctly.

#### Scenario: Dispatch is recorded

- **WHEN** an agent posts a subagent dispatch event with an identifier, a kind
  and a task
- **THEN** the response is a success and the agent gains a running subagent
  carrying that kind and task

#### Scenario: Dispatch without a kind

- **WHEN** a dispatch event carries an identifier and a task but no kind
- **THEN** the subagent is recorded with its kind unstated, and the request
  succeeds

#### Scenario: Completion is recorded

- **WHEN** an agent posts a completion event for a subagent it dispatched
- **THEN** that subagent's state becomes finished, or failed with its reason

#### Scenario: A subagent event does not change activity status

- **WHEN** an agent posts a subagent dispatch event while it is Working
- **THEN** its activity status is still Working, and no desktop notification is
  raised

#### Scenario: Missing identifier is rejected

- **WHEN** a subagent event omits the subagent identifier
- **THEN** the response is 400 and no state changes

#### Scenario: Completion for an unknown subagent is ignored

- **WHEN** a completion event names an identifier with no recorded dispatch
- **THEN** the response is a success and no subagent record is created

#### Scenario: An agent type that does not post these events

- **WHEN** an agent never posts a subagent event
- **THEN** it holds no subagent records, and its type's declared reporting path
  is what decides whether that reads as "none dispatched" or "cannot tell"
