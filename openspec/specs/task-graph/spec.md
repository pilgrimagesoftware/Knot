# task-graph Specification

## Purpose
Defines the unit of work that sits between a single prompt and a whole job: a
directed acyclic graph of tasks with assignees and dependencies, the states a
task moves through, when an agent must commit such a plan before dispatching
work, and the dependency gate that stops a task being dispatched before what
it depends on is finished.

## Requirements

### Requirement: Task and graph shape

A task SHALL carry an identifier unique within its graph, a goal in plain
text, an optional assignee, a list of dependency identifiers, and a state. An
assignee SHALL be either an agent the owner may see, or a set of capability
tags to be resolved against the registry at dispatch time; a task MAY have no
assignee, since a plan is allowed to record work before deciding who does it.

A graph SHALL be owned by exactly one agent, and that agent SHALL have at
most one committed graph at a time. A graph SHALL be runtime state: it SHALL
NOT persist across an app restart, and it SHALL be discarded when its owning
agent is removed. This matches how messages behave (`mcp-messaging`); a plan
whose agents are gone is not a plan.

Divergence from the Swift reference: the Swift app has no task object at all.
Delegated work exists only as free text in a message, and nothing records
whether it finished.

#### Scenario: A task may be planned before it is assigned

- **WHEN** a graph is committed containing a task with a goal, dependencies
  and no assignee
- **THEN** the graph is accepted and that task is held unassigned

#### Scenario: A graph does not survive a restart

- **WHEN** an agent has a committed graph and the app is restarted
- **THEN** that agent has no committed graph

#### Scenario: Removing the owner discards the graph

- **WHEN** the agent owning a committed graph is removed
- **THEN** its graph is discarded and its tasks can no longer be dispatched

### Requirement: A graph is rejected unless it is acyclic and closed

Committing a graph SHALL be rejected when any dependency names a task absent
from the same graph, when any task depends on itself, when the dependencies
form a cycle, or when the graph exceeds the system's task limit. A rejection
SHALL name the offending task or edge, and SHALL leave any previously
committed graph in place unchanged, so a bad plan cannot destroy a good one.

Validation SHALL happen at commit, not at dispatch. A plan that cannot be
executed to completion is not worth dispatching the first task of.

#### Scenario: A cycle is refused

- **WHEN** a graph is committed in which task A depends on B and B depends
  on A
- **THEN** the commit is rejected, the rejection names the cycle, and the
  owner's previous graph is untouched

#### Scenario: A dangling dependency is refused

- **WHEN** a committed graph contains a task depending on an identifier no
  task in that graph carries
- **THEN** the commit is rejected and the rejection names that identifier

#### Scenario: A self-dependency is refused

- **WHEN** a task lists its own identifier among its dependencies
- **THEN** the commit is rejected

### Requirement: Task state machine

Every task SHALL hold exactly one of `pending`, `ready`, `dispatched`,
`done`, `failed` or `blocked`.

On commit, a task with no dependencies SHALL be `ready` and every other task
SHALL be `pending`. A `pending` task SHALL become `ready` as soon as all its
dependencies are `done`. A task SHALL become `dispatched` only by being
dispatched, and SHALL leave `dispatched` only for `done` or `failed`, each
reported by the owner. When a task becomes `failed`, every task that depends
on it, directly or transitively, SHALL become `blocked`, since the work they
were waiting on did not happen.

A `blocked` task SHALL NOT become `ready` on its own. It returns to the graph
only through a new commit that changes what it depends on.

#### Scenario: Completing the last dependency readies a task

- **WHEN** a task depends on two others and the second is reported `done`
- **THEN** that task becomes `ready`

#### Scenario: A failure blocks everything downstream

- **WHEN** a task is reported `failed` and two further tasks depend on it,
  one directly and one through the first
- **THEN** both become `blocked`

#### Scenario: A blocked task does not recover by itself

- **WHEN** a task is `blocked` and its sibling tasks complete
- **THEN** it stays `blocked`

### Requirement: Nontrivial work requires a committed plan

Work is nontrivial when it will be split across more than one agent, or into
more than one task for a single agent. Nontrivial work SHALL be dispatched
only from a committed graph.

Work that is a single task for a single agent SHALL remain dispatchable as an
ordinary message, with no graph required. Forcing a graph around one-shot
work would make the plan ceremony rather than a record of ordering, and
ordering is the only thing a graph exists to capture.

#### Scenario: A single hand-off needs no plan

- **WHEN** an agent sends one agent one piece of work
- **THEN** the message is delivered under the existing messaging rules and no
  graph is required

#### Scenario: A fan-out without a plan is refused

- **WHEN** an agent attempts to dispatch tasks to two agents with no
  committed graph
- **THEN** the dispatch is refused and the refusal states that a plan must be
  committed first

### Requirement: Dispatch is gated on dependencies

Dispatching a task SHALL be refused unless the caller owns a committed graph
containing that task and the task is `ready`. A refusal SHALL name the
dependencies that are not yet `done`, so the caller learns what it is waiting
for rather than only that it may not proceed.

Dispatch SHALL deliver the task's goal to its assignee through the existing
messaging rules, unchanged: the sender must be registered, the recipient must
be in the sender's workspace, shell agents cannot receive, and companion
routing applies. A dispatch the messaging rules reject SHALL leave the task
`ready` rather than marking it `dispatched`, and SHALL return the messaging
rejection text.

Where the assignee is a set of capability tags rather than an agent, dispatch
SHALL resolve it against the registry and use the first candidate, and SHALL
be refused with an explanatory message when no candidate matches.

#### Scenario: An unmet dependency names itself

- **WHEN** a task whose dependency is still `dispatched` is dispatched
- **THEN** the dispatch is refused and the refusal names that dependency

#### Scenario: A rejected delivery does not consume the task

- **WHEN** a ready task is dispatched to an agent in another workspace
- **THEN** the messaging rejection is returned and the task is still `ready`

#### Scenario: A tag assignee resolves at dispatch time

- **WHEN** a ready task assigned to the tag `code-review` is dispatched and
  one visible agent carries that tag
- **THEN** the goal is delivered to that agent and the task becomes
  `dispatched`

#### Scenario: No candidate for a tag assignee

- **WHEN** a ready task assigned to a tag no visible agent carries is
  dispatched
- **THEN** the dispatch is refused with a message saying no agent matches,
  and the task is still `ready`

### Requirement: Re-planning preserves work already in flight

Committing a graph while one is already committed SHALL carry over the state
of every task whose identifier appears in both, and SHALL NOT cancel, recall
or duplicate a task that is `dispatched`. Tasks only in the old graph SHALL
be dropped; tasks only in the new graph SHALL start at `pending` or `ready`
by the usual rule.

A plan can be revised mid-flight — that is the point of holding it as data —
but revising it SHALL NOT silently re-send work an agent is already doing.

#### Scenario: A dispatched task keeps its state across a re-plan

- **WHEN** a new graph is committed that still contains a task currently
  `dispatched`
- **THEN** that task is still `dispatched`, and nothing is re-sent to its
  assignee

#### Scenario: A completed task is not re-run

- **WHEN** a new graph is committed containing a task already `done`
- **THEN** that task is still `done` and is not dispatched again

### Requirement: A committed plan is shown to the user

Committing a graph SHALL make the plan visible to the user in the owning
agent's panel, as a diagram of the tasks and their dependencies with each
task's current state. The user SHALL be able to see the shape of the work
before it fans out, rather than inferring it from messages after the fact.

This SHALL reuse the panel diagram surface agents already have
(`mcp-tools`' `view-mermaid`); no new window is introduced.

#### Scenario: The plan appears on commit

- **WHEN** an agent commits a graph of four tasks
- **THEN** its panel shows those four tasks and the dependency edges between
  them

#### Scenario: The diagram tracks state

- **WHEN** a task in a shown plan is reported `done`
- **THEN** the panel's diagram shows that task as `done`
