# Design

## Context

See `proposal.md` - Why. Three facts about the current tree shape the
approach:

- **Status is already observable.** `agents_snapshot()` in
  `crates/knot-mcp-tools/src/lib.rs` clones the live `Vec<Agent>` and is
  already served over `GET /api/v1/agent/status` and through `list-agents`.
  The registry needs no new state plumbing for its `status` field.
- **Nothing resembling a task exists.** There is no `Task`, `Job`, `Dag` or
  dependency edge anywhere in `crates/`. The nearest neighbours are
  `knot_messaging::Message` (one free-text hand-off, no ordering) and
  `QueuedPanelPrompt` (prompts waiting for *one* agent's session). Neither
  can carry a dependency, so the graph is genuinely new code rather than an
  extension of either.
- **Two files are near their limits.** `crates/knot-mcp-tools/src/lib.rs` is
  580 lines against the project's 700-line cap, and it holds both the tool
  catalogue and the dispatch `match`. Five more tools do not fit.

The registry fields themselves are the smaller half of the work: three
`#[serde(default)]` fields on two records. The graph is the half with design
decisions in it.

## Goals / Non-Goals

**Goals:**

- Keep the graph testable without an MCP server, a UI, or an async runtime.
- Make the registry a *view*, so there is exactly one place an agent's
  description and tags live.
- Put the plan-before-dispatch rule in the tool contract, where it binds,
  rather than in prompt text, where it advises.
- Land additively: an existing `settings.json` loads and an existing agent
  keeps working with no migration step.

**Non-Goals:**

- Scheduling. Nothing picks a ready task and dispatches it automatically; the
  orchestrator calls `dispatch-task` itself. Auto-dispatch would need a
  policy for concurrency and pre-emption that no requirement here asks for.
- Cross-agent graph visibility. A graph belongs to its owner; other agents
  see dispatched goals as ordinary messages.
- Reworking `agent_type`. See `proposal.md` - Non-goals.

## Decisions

### A new `knot-tasks` crate, not a module in `knot-agents`

The graph goes in a standalone crate alongside `knot-git` and
`knot-discovery`: no async runtime, no UI types, no MCP types, no dependency
on the agent store. It holds the node type, cycle detection, the state
machine and the dependency gate, and it refers to agents only by `Uuid`.

*Alternative considered:* a module inside `knot-agents`. Rejected because the
graph would then be able to reach the agent store, and the tempting shortcut
- resolving a capability tag from inside the state machine - is exactly the
coupling that makes the state machine untestable. Keeping the crate ignorant
of what an agent *is* forces tag resolution to happen at the call site, which
is where it belongs (see below).

*Alternative considered:* a module inside `knot-mcp-tools`. Rejected on the
580-line problem above and because it would tie graph tests to tool-call
plumbing.

### Tag resolution happens in the MCP layer, not in the graph

`knot-tasks` stores an assignee as either a resolved `Uuid` or an unresolved
tag set, and returns the tag set to the caller at dispatch. The MCP handler
asks the registry for candidates, picks the first, and tells the graph which
agent it used. The graph never queries anything.

This keeps a single rule about *when* resolution happens - at dispatch, not
at planning - visible in one place, and it means a re-plan cannot quietly
re-resolve an assignee behind a dispatched task.

### The registry is a projection, not a stored collection

There is no `registry` entry in `Settings`. A registry candidate is computed
on demand from `agents_snapshot()` plus `Settings::bench_agents`, with the
three new fields read off each record. `settings.json` gains no new
top-level key.

*Alternative considered:* a separate registry collection keyed by agent id.
Rejected: it would be a second copy of an agent's description, and the pair
would diverge the first time an agent was edited outside the registry path.

### Capability tags are opaque strings; cost tier is an enum

The project's rule is that closed vocabularies are enums rather than strings
matched with a `_ => default` arm
(`.claude/rules/rust-structure.md`). The two fields land on opposite sides of
it deliberately:

- `capabilities` is an **open** vocabulary - the whole point is that a new
  kind of agent is added by tagging it. It is a `BTreeSet<String>`,
  normalized on write (trimmed, lowercased) so matching is exact equality on
  the normalized form. Nothing anywhere `match`es a tag, so the rule's actual
  hazard - a silent `_ => default` arm - cannot arise.
- `cost_tier` is a **closed** vocabulary of three values, so it is a
  `CostTier` enum in `knot-core` with `Display`/`FromStr`, and ranking uses
  its declaration order rather than a lookup table of magic numbers.

### Declared cost, not measured cost

ACP already reports `SessionUpdate::Usage { used, size }`, surfaced as
"Context usage". That is how full a context window is, not what a session
costs; ranking on it would call an agent with a long conversation
"expensive". `cost_tier` is therefore a value the user sets, honest about
being a hint. Metering real usage is a later change with its own spec.

### The gate is in the tool, and bypass stays possible

`dispatch-task` refuses a task whose dependencies are unmet. `send-message`
is untouched, so an agent *can* still fan out work by hand and skip the
graph. Closing that hole would mean gating `send-message` itself, which would
break every existing agent interaction to enforce a rule only orchestrators
need. Instead: planned dispatch is the path with tool support, and the plan
diagram makes an unplanned fan-out visible by its absence.

### Splitting `knot-mcp-tools`

Before adding tools, the catalogue and dispatch `match` move out of
`lib.rs` into a `catalog.rs`, and the five new handlers go in a `tasks/`
module beside the existing `agents/`, `messaging.rs`, `panels.rs`,
`repos.rs`. This is a precondition, not a cleanup to do afterwards - the file
is over the cap the moment the tools are added.

### The plan diagram wires up an already-present dead end

`view-mermaid` sets `Agent::mermaid_source`, and the field's own doc comment
says it is "not yet consumed by any UI" - confirmed: no file under
`crates/knot/` reads it. So `task-graph`'s "A committed plan is shown to the
user" requires rendering that panel state, which this change must do.

The graph emits mermaid text on commit and on each state change, writes it
through the same panel-state path `view-mermaid` uses, and the panel renders
it. One consequence worth naming: this also makes the existing `view-mermaid`
tool work for the first time, for every agent, not just orchestrators.

## Risks / Trade-offs

- **The gate is advisory in practice.** An agent that ignores `plan-tasks`
  and calls `send-message` five times gets its work done with no plan. →
  Accepted, per the decision above. The shipped Orchestrator persona
  instructs the planned path, and the user sees no diagram when it is
  skipped, which is the signal that something went around it.
- **"Nontrivial" is a judgement the model makes.** The system can refuse an
  ungated `dispatch-task`, but it cannot tell that three `send-message` calls
  were one job. → The definition in `task-graph` is written to be checkable
  where it can be (dispatch), and instructive where it cannot.
- **Rendering mermaid is UI work in a change that is otherwise data and
  tools.** → It is scoped to displaying existing panel state, not a new
  window, and it is the last task group so it can be deferred to a follow-up
  without stranding anything else if it proves larger than it looks.
- **Cost tiers are self-reported and will be wrong sometimes.** → Ranking is
  documented as a recommendation; the orchestrator still chooses, and status
  outranks cost so a wrong tier costs at most a suboptimal pick.
- **A downgrade loses the new fields.** `Settings` does not use
  `deny_unknown_fields`, so an older build reads a newer file, ignores the
  three fields, and drops them on its next write. → Additive and recoverable
  by re-entering them; called out rather than guarded, since the app is
  single-user and not versioned across machines.
- **Tag typos silently produce no candidates.** `code-reveiw` matches
  nothing. → `describe-agents` with no tags lists every tag in use, so the
  vocabulary is discoverable; an empty result is a success, not an error, so
  the caller can react.

## Migration Plan

Additive, single-step, no data migration:

1. `CostTier` and the three record fields land with `#[serde(default)]`. An
   existing `settings.json` loads unchanged; every agent reads back as
   described in `agent-lifecycle`'s legacy-record scenario.
2. `knot-mcp-tools` splits before the new tools are added, so no commit in
   the sequence leaves a file over the 700-line cap.
3. `knot-tasks` lands with its tests before any tool references it.
4. Tools, then editor fields, then the panel diagram. Each group is
   independently shippable; stopping after any of them leaves a working app.

Rollback is reverting the commits: nothing writes state an older build
cannot read, and no graph outlives the process.
