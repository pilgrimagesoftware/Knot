# Spec Delta

## Purpose

Gives Knot a runtime representation of the agents an agent delegates to — the
subagent a coding agent spawns from within its own turn and waits on — so that
delegated work is visible, named and timed from the agent's pane instead of
being the one thing an idle-looking agent cannot be asked about. The Swift
reference app has no equivalent; this is new behavior in the Rust port.

## ADDED Requirements

### Requirement: A subagent is a typed record, not free text

A subagent SHALL be represented as a record carrying an identifier unique
within its parent agent's session, the kind of subagent that was dispatched,
the task it was given, the instant it started, and its state.

The kind SHALL be whatever the reporting agent named — it is an open
vocabulary, because the set of subagent kinds is the user's own configuration,
not Knot's. Where the report carries no kind, the record SHALL say the kind is
unstated rather than substitute a default one.

The state SHALL be a closed vocabulary — running, finished, or failed — with
no default arm. A report that does not resolve to one of these SHALL NOT
produce a record.

#### Scenario: A dispatched subagent becomes a record

- **WHEN** an agent dispatches a subagent of a named kind with a task
- **THEN** a record exists carrying that kind, that task, its start instant and
  the running state

#### Scenario: A subagent with no stated kind

- **WHEN** a report names a task but no subagent kind
- **THEN** the record's kind reads as unstated, and is not filled in with a
  general-purpose or default kind

#### Scenario: An unrecognizable report produces nothing

- **WHEN** a report cannot be resolved to a subagent kind, task and state
- **THEN** no record is created, and the failure does not disturb the records
  already held

### Requirement: A subagent's lifecycle is observed, not sampled

A subagent record SHALL be created when its dispatch is reported and SHALL move
to finished or failed when its completion is reported. Its elapsed time SHALL
run from its start instant to its completion, and from its start instant to now
while it is running.

A record SHALL persist after completion until the parent agent's turn ends, so
that a subagent which started and finished between two glances at the pane is
still accounted for. When the parent agent's turn ends, or the agent stops, its
records SHALL be discarded.

A completion report naming no record SHALL be ignored rather than create one.

#### Scenario: A subagent that finishes stays listed

- **WHEN** a subagent finishes while its parent agent's turn is still running
- **THEN** its record remains, showing the finished state and the elapsed time
  it took, rather than disappearing

#### Scenario: Elapsed time advances while running

- **WHEN** a subagent has been running for four minutes
- **THEN** its record's elapsed time reads as four minutes without any new
  report having arrived

#### Scenario: The turn ending clears the records

- **WHEN** the parent agent's turn ends
- **THEN** its subagent records are discarded

#### Scenario: A stopped agent holds no records

- **WHEN** an agent is deactivated or its session process exits
- **THEN** its subagent records are discarded

#### Scenario: A completion for an unknown subagent

- **WHEN** a completion is reported for a subagent Knot never saw dispatched
- **THEN** it is ignored, and no finished record appears

### Requirement: A failed subagent is distinguished from a finished one

A subagent whose completion is reported as an error SHALL be recorded as
failed, carrying the reported reason where one is given. A failed subagent
SHALL NOT be recorded as finished, and SHALL NOT be silently dropped.

#### Scenario: A failure is retained with its reason

- **WHEN** a subagent's completion reports an error with a reason
- **THEN** its record reads as failed and carries that reason

#### Scenario: A failure with no stated reason

- **WHEN** a subagent's completion reports an error with no reason
- **THEN** its record reads as failed with no reason, rather than as finished

### Requirement: Subagents reach Knot by the agent's own reporting

Subagent records SHALL be produced only from what the agent itself reports.
The system SHALL NOT infer a subagent from the process table, from terminal
output, or from any heuristic over the agent's prose.

For an agent whose session speaks the agent-client protocol, the report SHALL
be drawn from the tool-call stream that session already delivers, using the
tool call's raw input to identify a delegation and read its kind and task.

For an agent running in a pseudo-terminal, the report SHALL be drawn from the
hook events the agent posts to Knot's existing hook route.

#### Scenario: A protocol session reports through its tool calls

- **WHEN** a protocol-backed agent issues a tool call whose raw input
  identifies it as a delegation
- **THEN** a subagent record is created from that call, and the call's
  completion moves the record to finished or failed

#### Scenario: A terminal session reports through hooks

- **WHEN** a terminal-backed agent posts a hook event reporting a subagent
  dispatch
- **THEN** a subagent record is created from that event

#### Scenario: No inference from the process table

- **WHEN** a process appears under an agent's session root whose command
  resembles a delegated task
- **THEN** no subagent record is created from it

### Requirement: An agent type states whether it can report subagents

Each agent type SHALL declare how it reports subagents: through its protocol
tool calls, through hook events, or not at all. A type that declares neither
SHALL be treated as unable to report, and an agent of that type SHALL NOT be
described as having dispatched nothing.

A type that can report but from which nothing has yet been received SHALL be
described as having dispatched nothing, which is a different answer.

#### Scenario: A type with no reporting path

- **WHEN** an agent whose type declares no reporting path is running
- **THEN** its subagent state reads as unavailable, not as empty

#### Scenario: A reporting type that has dispatched nothing

- **WHEN** an agent whose type can report subagents has dispatched none
- **THEN** its subagent state reads as empty, not as unavailable

#### Scenario: A shell agent reports nothing

- **WHEN** the agent is a shell agent
- **THEN** its subagent state reads as unavailable, and no reporting path is
  attempted for it

### Requirement: Subagents are recorded flat, and are not attributed processes

Every subagent of one parent agent SHALL be held at one level. A subagent that
itself reports a delegation SHALL be recorded as another subagent of the same
parent rather than nested beneath the one that dispatched it.

The system SHALL NOT associate any operating-system process with a subagent.
A process a subagent runs descends from the same session root as everything
else the parent agent runs, and the system SHALL NOT claim an attribution it
cannot establish.

#### Scenario: A subagent's own delegation is recorded flat

- **WHEN** a subagent reports dispatching a further subagent
- **THEN** both are recorded as subagents of the same parent agent, with no
  nesting between them

#### Scenario: No process is claimed for a subagent

- **WHEN** a subagent runs a command that appears in the parent agent's
  descendant process list
- **THEN** that process is not associated with the subagent, and appears only
  among the parent agent's processes

### Requirement: Subagent state never blocks a frame

Subagent records SHALL be built from reports as they arrive, off the render
path, and a render SHALL read only what has already been recorded. No report
parsing, and no waiting on a report, SHALL occur during a render pass.

A record that changes between two frames SHALL reach a frame — a new subagent,
a completion, or a discard SHALL cause the pane to repaint rather than wait for
an unrelated repaint to carry it.

#### Scenario: No parsing during a frame

- **WHEN** an agent's pane re-renders, including on every keystroke
- **THEN** no report is parsed as part of that render

#### Scenario: A completion reaches the screen

- **WHEN** a subagent completes while its parent agent's pane is shown
- **THEN** the pane repaints to show the completed state without further user
  action
