# mcp-tools Specification

## Purpose
Defines the concrete MCP tool catalog Knot exposes to agents: each tool's
name, required and optional inputs, success payload, and failure behavior.
Routing rules for messaging tools are owned by `mcp-messaging`; this spec fixes
the tool surface itself.

## Requirements

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

### Requirement: Missing required argument is a tool error

When a tool call omits a required argument, the handler SHALL return a tool
result with `isError` true and text naming the missing argument. It SHALL NOT
raise a transport-level error.

#### Scenario: send-message without content

- **WHEN** `send-message` is called with `from` and `to` but no `content`
- **THEN** the result has `isError` true and names the missing `content`

### Requirement: register-agent

`register-agent` SHALL require `agentId` and accept optional `sessionId`. On
success it SHALL mark the agent registered, associate the session id when
given, and return the unread-message count and the list of knot members
visible to the caller.

#### Scenario: Register returns roster

- **WHEN** a known agent calls `register-agent` with its id
- **THEN** it is marked registered and the result includes the current knot
  members

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

### Requirement: send-message, check-messages, broadcast-message

`send-message` SHALL require `from`, `to`, `content`. `check-messages` SHALL
require `agentId` and accept optional `markAsRead` (default true).
`broadcast-message` SHALL require `from`, `content`. Their delivery, routing,
and rejection semantics are defined by `mcp-messaging`; on rejection the tool
result SHALL carry the rejection string with `isError` true.

#### Scenario: Rejected send surfaces the reason

- **WHEN** `send-message` targets a shell agent
- **THEN** the tool result text is "Cannot send messages to shell agents" with
  `isError` true

### Requirement: list-repos and list-worktrees

`list-repos` SHALL take no arguments and return repositories discovered under
the configured source folder, each with its name and worktrees.
`list-worktrees` SHALL require `repoPath` and return that repository's
worktrees as name and absolute path.

#### Scenario: list-worktrees for a repo

- **WHEN** `list-worktrees` is called with a valid `repoPath`
- **THEN** the result lists each worktree's name and path

### Requirement: create-agent

`create-agent` SHALL require the caller's `agentId`. It SHALL accept
`benchAgentId` (deploy a bench template; then name/agentType/repoPath are
optional), or otherwise require `name`, `agentType`, and `repoPath`. Optional
inputs: `icon`, `createWorktree` with `branchName`, `companion`, `command`
(shell only), `personaId`. On success it SHALL create the agent attributed to
the caller and return the new agent id.

#### Scenario: Create from explicit fields

- **WHEN** `create-agent` is called with `name`, `agentType`, `repoPath`
- **THEN** an agent is created with `created_by` set to the caller and the new
  id is returned

#### Scenario: Create with a new worktree

- **WHEN** `create-agent` sets `createWorktree` true without `branchName`
- **THEN** the result is an error naming the missing `branchName`

### Requirement: close-agent

`close-agent` SHALL require the caller's `agentId` and a `target`. It SHALL
close the target only if the caller created it; otherwise it SHALL fail with a
message stating the caller may only close agents it created.

#### Scenario: Cannot close a user-created agent

- **WHEN** an agent calls `close-agent` targeting an agent it did not create
- **THEN** the call fails and nothing is closed

### Requirement: create-worktree

`create-worktree` SHALL require `repoPath` and a non-empty `branchName`, create
a worktree on a new branch, and return the new worktree path on success.

#### Scenario: Empty branch name

- **WHEN** `create-worktree` is called with an empty `branchName`
- **THEN** the result is an error and no worktree is created

### Requirement: set-status

`set-status` SHALL require the caller's `agentId` and a `status` string. It
SHALL set the agent's human-readable status text; an empty string SHALL clear
it. This status is distinct from the automatic state machine value.

#### Scenario: Clear status

- **WHEN** `set-status` is called with an empty `status`
- **THEN** the agent's status text becomes empty and its state is unchanged

### Requirement: display-markdown and view-mermaid

`display-markdown` SHALL require `agentId` and `filePath` and accept optional
`maximized`. `view-mermaid` SHALL require `agentId` and `source` and accept
optional `title`. Each SHALL update the target agent's panel state and return a
success indicator. `display-markdown` SHALL record a history of shown files,
most recent first.

#### Scenario: Show a markdown file

- **WHEN** `display-markdown` is called with a valid `filePath`
- **THEN** the agent's markdown panel targets that file and the file is
  prepended to its file history

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
