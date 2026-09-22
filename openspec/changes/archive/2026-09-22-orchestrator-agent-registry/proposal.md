# Proposal

## Why

Knot can already create, message and close agents, but nothing in the system
describes what an agent is *for*. `list-agents` returns an id, a name, a
folder and a status string — enough to address an agent, not enough to choose
one. Any agent acting as an orchestrator therefore has to carry its team
roster in its own prompt text: which teammates exist, what each is good at,
what each costs to run. That roster is hand-written, goes stale the moment an
agent is added or removed, and cannot be read by anything but the model that
was handed it.

The same gap shows up on the dispatch side. There is no unit of work in the
system between "a prompt sent to one agent" and "the whole job". An
orchestrator splitting a job across a research agent, two coding agents and a
reviewer has nowhere to record that the reviewer runs after the coders, so it
either serialises everything or dispatches in parallel and hopes. Knot's own
existing safeguards work at the message level (`mcp-messaging`) and know
nothing about ordering.

Closing both gaps at once gives the foundation the roster hack is standing in
for: agent kinds become data the app owns, and a plan becomes an object the
app can validate and show, rather than an intention living in one model's
context window.

## What Changes

- **Agent registry.** Every agent — a live one and a bench template that
  could become one — gains declarative metadata: a one-line `description`, a
  set of free-form `capability` tags, the `tools` (agent type plus its
  adapter's declared tool surface) it can reach, a declared `cost_tier`, and
  its live `status`. Descriptions, capabilities and cost tier are durable and
  editable in the agent editor and the bench; tools and status are derived.
- **Registry queries over MCP.** `list-agents` grows the new fields.
  A new `describe-agents` tool answers "who can do X" by capability tag,
  returning candidates ranked cheapest-first, across both live agents and
  deployable bench templates. Existing workspace and companion visibility
  rules apply unchanged.
- **Task graph.** A new runtime object: a directed acyclic graph of tasks,
  each with a goal, an optional assignee resolved against the registry, a
  dependency list, and a state (`pending`, `ready`, `dispatched`, `done`,
  `failed`, `blocked`). Cycles are rejected at submission.
- **Plan before dispatch.** New MCP tools `plan-tasks`, `dispatch-task`,
  `complete-task` and `task-status`. Work is *nontrivial* when it spans more
  than one agent or more than one task; nontrivial work SHALL go through a
  committed graph, and `dispatch-task` SHALL refuse a task whose dependencies
  are not `done`. A single task for a single agent stays a plain
  `send-message` — the graph is not made mandatory for one-shot work.
- **The roster becomes a query, not prompt text.** An orchestrator learns
  its team by calling `describe-agents`, the way agents already discover each
  other with `list-agents` — never from a teammate list written into its
  instructions. Adding a coding, research, infrastructure, review or testing
  agent changes what the next query returns, with no prompt edited anywhere.
  A shipped Orchestrator persona carries the *method* (query the registry,
  plan, then dispatch) and deliberately names no teammate, so its text stays
  correct as the team changes.
- **The plan is visible.** The orchestrator renders its committed graph
  through the existing `view-mermaid` tool, so the user sees the plan in the
  orchestrator's panel before work fans out. No new window.

### Non-goals

- Measuring real token or dollar spend. `cost_tier` is a declared class
  (`low` / `medium` / `high`), not telemetry. Metering ACP usage reports is a
  later change.
- Persisting task graphs. Like messages (`mcp-messaging`), a graph is
  in-memory and does not survive an app restart.
- A general workflow engine. Dependencies only — no loops, conditionals,
  retries, or timers.
- Automatic assignment. The registry ranks candidates; the orchestrator still
  chooses, and a task may be planned with no assignee.
- Changing who may message whom. Workspace and companion routing rules in
  `mcp-messaging` are untouched, and dispatch delivers through them unchanged.
- Turning `agent_type` into an enum. It is a bare `String` today, in breach of
  the project's own closed-vocabulary rule, but capability tags are the axis
  this change makes extensible; the type field is left exactly as it is.

## Capabilities

### New Capabilities

- `agent-registry`: the queryable catalogue of agent kinds and live agents —
  description, capability tags, reachable tools, declared cost tier, live
  status — its durable field set, how candidates are matched and ranked by
  capability, and how a registry-derived roster replaces a hard-coded one in
  an orchestrator's prompt.
- `task-graph`: the task DAG — node shape, dependency and acyclicity rules,
  the task state machine, what counts as nontrivial work, the dependency gate
  on dispatch, and how a committed plan is surfaced to the user.

### Modified Capabilities

- `mcp-tools`: the fixed `tools/list` catalogue gains `describe-agents`,
  `plan-tasks`, `dispatch-task`, `complete-task` and `task-status`, and
  `list-agents`' success payload gains the registry fields.
- `agent-lifecycle`: the durable agent field set gains `description`,
  `capabilities` and `cost_tier`; editing them SHALL NOT restart the agent,
  since none of them changes how the session runs.
- `settings-persistence`: the stored bench-agent record gains the same three
  fields, so a saved template carries its role, not just its folder.

## Impact

- `knot-core`: `SavedAgent` and `BenchAgent` records gain three fields
  (all `#[serde(default)]`, so existing settings files load unchanged); a new
  `CostTier` enum with `Display`/`FromStr` in the core error/type surface;
  new `consts.rs` entries for ranking and graph limits.
- `knot-agents`: `Agent` mirrors the new durable fields; a registry view that
  projects live agents plus bench templates into ranked candidates.
- **New crate `knot-tasks`**: the graph type, cycle detection, the state
  machine and the dependency gate. Runtime-agnostic and standalone, like
  `knot-git` — no UI, no MCP types, so it is testable without a server.
- `knot-mcp-tools`: five new tool handlers plus an extended `list-agents`
  payload; `knot-mcp` registers them. The 13-tool catalogue becomes 18.
- `knot`: agent-editor and bench UI fields for description, capabilities and
  cost tier; one further shipped system persona ("Orchestrator", static text
  naming no teammate, so `personas`' shipped-defaults and restore-defaults
  requirements are unaffected); all new copy through `knot_core::l10n::t`.
- The registry's live status needs no new plumbing: `agents_snapshot()`
  already serves the same state over `GET /api/v1/agent/status`.
- No new external dependencies. No change to the MCP transport, the ACP
  launch path, or git/worktree behavior.
