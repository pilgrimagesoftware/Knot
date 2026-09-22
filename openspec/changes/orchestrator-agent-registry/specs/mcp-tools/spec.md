# Spec Delta

## MODIFIED Requirements

### Requirement: Tool catalog

`tools/list` SHALL return exactly these tools: `register-agent`,
`list-agents`, `describe-agents`, `send-message`, `check-messages`,
`broadcast-message`, `list-repos`, `list-worktrees`, `create-agent`,
`close-agent`, `create-worktree`, `set-status`, `display-markdown`,
`view-mermaid`, `plan-tasks`, `dispatch-task`, `complete-task`,
`task-status`. Every tool SHALL declare a JSON object input schema with typed
properties and a required list.

#### Scenario: Catalog is complete and stable

- **WHEN** `tools/list` is called
- **THEN** all eighteen tools are present, each with a name, description, and
  object input schema

### Requirement: list-agents

`list-agents` SHALL require the caller's `agentId` and return, for each agent
the caller may see (its own workspace; companions only if owned by the caller),
the agent id, name, folder, status string, and registered flag, together with
that agent's registry fields: its description, its capability tags, the tools
it can reach, and its cost tier.

An agent with no registry metadata SHALL still be listed, with an empty
description, no tags, and cost tier `medium`, so the tool never hides an
agent for lacking a description.

#### Scenario: Caller sees own workspace

- **WHEN** an agent calls `list-agents`
- **THEN** the response lists agents in the caller's workspace and excludes
  companions the caller does not own

#### Scenario: Registry fields accompany each agent

- **WHEN** an agent calls `list-agents` and a visible agent is described as
  "Runs the test suite" with tag `testing` at cost tier `low`
- **THEN** that entry carries the description, the tag, the reachable tools,
  and cost tier `low`

#### Scenario: An undescribed agent is still listed

- **WHEN** a visible agent has no description and no tags
- **THEN** it appears with an empty description, no tags, and cost tier
  `medium`

## ADDED Requirements

### Requirement: describe-agents

`describe-agents` SHALL require the caller's `agentId` and accept an optional
`capabilities` array of tags and an optional `includeTemplates` flag
(default true). It SHALL return the registry candidates matching every
requested tag that the caller may see, ranked as `agent-registry` defines,
each carrying identifier, description, capability tags, reachable tools,
cost tier and status. Omitting `capabilities` SHALL return every visible
entry. No match SHALL return an empty list with `isError` false.

#### Scenario: Query by tag

- **WHEN** an agent calls `describe-agents` with `capabilities` of
  `["code-review"]`
- **THEN** the result lists only visible entries carrying that tag, ranked
  cheapest-idle-first

#### Scenario: Templates can be excluded

- **WHEN** the call passes `includeTemplates` false
- **THEN** bench templates are absent and only live agents are returned

#### Scenario: No match is not an error

- **WHEN** no visible entry carries the requested tag
- **THEN** the result is an empty candidate list with `isError` false

### Requirement: plan-tasks

`plan-tasks` SHALL require the caller's `agentId` and a `tasks` array. Each
task SHALL require an `id` unique within the call and a `goal`, and MAY carry
`assignee` (an agent id or name), `capabilities` (tags to resolve at dispatch
time), and `dependsOn` (ids within the same call). On success it SHALL commit
the graph for the caller and return each task's id and resulting state. On a
validation failure it SHALL return `isError` true with text naming the
offending task or edge, per `task-graph`.

#### Scenario: A valid plan is committed

- **WHEN** `plan-tasks` is called with three tasks, one depending on the
  other two
- **THEN** the call succeeds and reports the two independent tasks `ready`
  and the dependent one `pending`

#### Scenario: A cyclic plan is a tool error

- **WHEN** the submitted tasks form a dependency cycle
- **THEN** the result has `isError` true and names the cycle

### Requirement: dispatch-task

`dispatch-task` SHALL require the caller's `agentId` and a `taskId` in the
caller's committed graph. On success it SHALL deliver the task's goal to its
assignee under the messaging rules in `mcp-messaging`, mark the task
`dispatched`, and return the recipient's id. It SHALL return `isError` true,
leaving the task `ready`, when the caller has no committed graph, the task is
not `ready`, no candidate resolves for a tag assignee, or messaging rejects
the delivery — carrying in each case the reason, and for an unmet dependency
the ids that are not yet `done`.

#### Scenario: Ready task is delivered

- **WHEN** `dispatch-task` names a `ready` task assigned to a visible agent
  in the caller's workspace
- **THEN** the goal is delivered, the task becomes `dispatched`, and the
  recipient id is returned

#### Scenario: Unmet dependency is reported

- **WHEN** `dispatch-task` names a task whose dependency is not `done`
- **THEN** the result has `isError` true and names that dependency

### Requirement: complete-task and task-status

`complete-task` SHALL require the caller's `agentId`, a `taskId` in the
caller's committed graph, and an `outcome` of `done` or `failed`, and MAY
carry a `note`. It SHALL apply the state transition and the downstream
blocking rule in `task-graph`, and return the ids that became `ready` and the
ids that became `blocked`.

`task-status` SHALL require the caller's `agentId` and return every task in
the caller's committed graph with its state, assignee and dependencies, or an
empty graph when the caller has committed none. It SHALL NOT be an error to
ask before planning.

#### Scenario: Completion readies a dependent task

- **WHEN** `complete-task` reports the last outstanding dependency `done`
- **THEN** the result names the dependent task among those now `ready`

#### Scenario: Failure blocks dependents

- **WHEN** `complete-task` reports a task `failed` and two tasks depend on it
- **THEN** the result names both among those now `blocked`

#### Scenario: Status before planning is empty, not an error

- **WHEN** an agent with no committed graph calls `task-status`
- **THEN** the result is an empty task list with `isError` false
