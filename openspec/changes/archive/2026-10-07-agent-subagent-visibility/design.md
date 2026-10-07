# Design

## Context

See `proposal.md` — Why. The constraints that shape the approach:

- **Two feeds arrive on two different threads, neither of which can notify
  GPUI.** ACP session events are folded into `PanelState` on the session's own
  task (`crates/knot/src/panel_session.rs:46`); hook POSTs are handled inside
  `McpToolCatalog`'s `AgentHookHandler` on an axum worker
  (`crates/knot-mcp-tools/src/lib.rs:168`). Both leave state behind for
  `repaint_poll_tick` to drain, the way `pull_request_states.take_changed()`
  and `drain_git_actions()` already do
  (`crates/knot/src/workspace_window/repaint.rs:59`).
- **ACP gives no tool name.** `SessionUpdate::ToolCallStart` carries `kind`
  (an icon hint) and `title` (prose the adapter writes)
  (`crates/knot-acp/src/protocol/mod.rs:169`). The protocol's `rawInput` is
  the only field that names what ran, and Knot currently parses none of it.
- **The processes section is a process-tree contract end to end.**
  `ProcessSection` holds `Option<Vec<DescendantProcess>>`
  (`crates/knot/src/agent_processes.rs:30`), the sampler publishes
  `BTreeMap<Uuid, Vec<DescendantProcess>>`, and every render helper takes a
  `&DescendantProcess`. A subagent has no PID, so it cannot be smuggled in as
  one of those.
- **`render/processes_pane.rs` is 412 lines** against a 700-line cap enforced
  by `make size-check`.

## Goals / Non-Goals

**Goals:**

- One representation of a subagent, whichever feed produced it, so the render
  and the header summary never branch on where a record came from.
- Recognition that fails closed: an unrecognized tool call or hook payload
  produces no record, never a wrong one.
- No new timer, no new subprocess, no new file read, nothing on the render
  path.

**Non-Goals (design-level, beyond the proposal's):**

- Persisting subagent records across an app restart. They describe work in
  flight; a restart ends the flight.
- A general tool-call-recognition framework. One recognizer per agent type for
  one question — is this a delegation — not an extensible rules engine.

## Decisions

### A new `knot-subagents` crate, not a module in `knot-core`

The model plus the per-type recognizers, with fixtures as the contract, the
way `knot-git` owns porcelain parsing and `knot-forge` owns `gh`. Both feeds'
payloads are plain JSON, so the whole recognizer surface is testable with no
GPUI, no ACP connection and no HTTP server.

*Alternative — a module under `knot-core`.* Rejected: `knot-core` is the
shared-types crate every other crate depends on, and this brings fixture files
and a parser that will change under it. It would also sit next to
`knot_core::import::subagents`, which reads persona definition files and has
nothing to do with this; two things called "subagents" in one crate is a trap
the file layout should not set.

### One registry keyed by agent id, two writers

`SubagentRegistry` behind an `Arc<Mutex<_>>`, held by both `McpToolCatalog`
(the hook writer) and the workspace window (the reader), carrying a changed
flag that `repaint_poll_tick` takes — one more branch in the `if` chain that
already has fourteen. The ACP fold writes to the same registry rather than
into `PanelState`.

*Alternative — keep ACP-sourced subagents in `PanelState`, which is already
per-agent and already covered by `panel_needs_repaint()`.* Rejected: the
render would then merge two collections with different shapes and different
lifetimes, and the collapsed header summary would have to know which view mode
the agent is in before it could count. The registry is the thing that makes
the spec's "subagents first, then processes" a single ordered read.

`take_changed()` has exactly one caller, in `repaint_poll_tick`. This is the
breakage `.claude/rules/rust-structure.md` names four times — a clearing read
reachable from a path that discards it — and the task list carries the test
for it.

### Recognition keys on the adapter's metadata marker, with raw input for the detail

**Revised during implementation.** The original decision said the tool name in
`rawInput` was the only available signal, on the reasoning that ACP's `kind` is
an icon hint and its `title` is prose. The first half of that holds; the
conclusion did not survive reading the adapter.

`@agentclientprotocol/claude-agent-acp` (v0.81.1) stamps every tool call with a
metadata envelope, and marks a delegation explicitly
(`dist/acp-agent.js`, `claudeCodeMetaFromToolUse`):

```js
return {
    toolName: toolUse.name,
    ...((toolUse.name === "Agent" || toolUse.name === "Task") && { subagent: true }),
    ...
};
```

So a recognizer keys on `_meta.claudeCode.subagent === true`, falling back to
`_meta.claudeCode.toolName` being `Task` or `Agent`. That is the adapter's own
answer to "is this a delegation", written for exactly this purpose — its
`native-subagents` module uses the same two checks to decide what to intercept.
`rawInput` then supplies the detail: `subagent_type`, `description`, `prompt`,
snake_case, verbatim from the model's tool-use input.

Keying on the marker rather than on a tool name inside `rawInput` means the
recognizer does not have to know that Claude Code's delegation tool is spelled
`Task` this month and `Agent` last month — the adapter already normalizes both
to one flag.

*Alternative — match `kind == "task"` or scan `title`.* Still rejected, and now
with the measurement: a delegation's `kind` is `"think"`, shared with ordinary
reasoning calls, and its `title` is `input.description || "Task"` — prose, and
the fallback is a bare English word.

### Channel B, not the adapter's dedicated subagent channel

The same adapter offers a first-class alternative, gated on a capability Knot
does not advertise (`dist/acp-subagents.d.ts`):

```ts
{ sessionUpdate: "subagent_spawned", subagentSessionId, name, task,
  capabilities: { cancel?, close? } }
{ sessionUpdate: "subagent_state_update", subagentSessionId,
  state: "completed" | "failed" | "cancelled" | "disconnected" }
```

Cleaner data, four states instead of two, and a `cancel` capability that would
make a genuine terminate action possible. Not taken, for two reasons.

It is an unshipped draft. The file says so: *"Temporary typed surface for
agentclientprotocol/agent-client-protocol#1992. The wire contract is already
defined by the ACP draft, but the published TypeScript SDK does not contain it
yet."* Pinning a shipped feature to a protocol revision that has not landed
buys a nicer shape for an unknown amount of churn.

And advertising the capability changes traffic beyond this change's scope. Once
a client negotiates it, `NativeSubagentRuntime.route()` suppresses the
delegation's own tool call and rescopes the subagent's nested tool calls onto a
synthetic `subagentSessionId` — a child session `acp-panel-ui` does not model.
Adopting the channel therefore means solving the panel transcript's
child-session problem first, which is its own change.

**Consequences to hold onto.** Two requirements in this change are correct only
for the channel being built, and must be revisited with the other one:

- *No terminate action on a subagent row.* True here: the tool-call path offers
  no cancellation. Not true on the dedicated channel, which advertises one.
- *`Outcome` is two-valued.* Correct here — a delegation's `tool_call_update`
  carries `status: "completed" | "failed"` and nothing else. The dedicated
  channel's four states would need `Cancelled` and `Disconnected` added, and
  the spec's closed vocabulary widened with them.

Tracked as a follow-up issue rather than an open question, because adopting it
changes specs rather than filling a gap in this one.

### Identity is the reporter's, not Knot's

`SubagentId(String)`, scoped within one agent: the ACP `toolCallId` for the
protocol feed, the posted identifier for the hook feed. Knot never mints one.

*Alternative — a Knot-side `Uuid` per record.* Rejected: the completion report
carries the reporter's identifier and nothing else, so a Knot-minted id would
need a side table mapping one to the other, which is the reporter's identifier
with extra steps.

### A turn ending is what clears the records, and each feed has its own signal

Panel: `SessionUpdate::TurnEnd`, already folded. Terminal: the Claude `Stop`
hook, which Knot already receives as a status of `idle`
(`openspec/specs/agent-hooks/spec.md` — "Claude activity status"). No new
event is needed for the clear in either feed; only the dispatch and completion
events are new.

An agent that dies mid-turn sends neither. Records are therefore also
discarded when the agent stops and when its session ends — both already
observed by the window (`remove_agent`, `SessionEvent::Ended`).

### Finished records are capped per agent

Running records are never dropped. Finished and failed ones are retained up to
a cap in `consts.rs`, oldest discarded first. A long autonomous turn can
dispatch dozens; the section is a status view, not an audit log, and an
unbounded `Vec` behind a mutex read every frame is the wrong shape for one.

### `agent_type` gains a closed `SubagentReporting` column

`None | ToolCalls | Hooks | Either`, resolved against the agent's view mode at
read time — a type can report through its protocol in Panel mode and through
hooks in Terminal mode, and those are different answers for the same type.
`None` is what makes the spec's "unavailable" distinct from "dispatched none",
and shell agents resolve to it structurally.

**Sequencing matters here.** The hook emitter is a Claude plugin outside this
repo (see the proposal's non-goals). Claude therefore ships as `ToolCalls`,
not `Either`: a Terminal-mode Claude agent then reads as *unavailable*, which
is true, instead of *dispatched none*, which would be a confident lie. Flipping
the column to `Either` is a one-line follow-up once the plugin emits.

### Elapsed time is rendered from a stored start instant, reusing the existing runtime keys

A record stores `Instant` and the row formats `now - start` through the same
`processes.runtime_*` shapes the process rows use. Reading the clock during a
render is what `working_indicator` already does; the coarse units mean a
running row cannot visibly drift between the repaints the spinner poll already
drives while an agent is Working.

*Alternative — store an elapsed `Duration` refreshed by a tick.* Rejected: a
new timer for a value the clock already has.

### `render/processes_pane.rs` splits before the second row kind lands

Into a `processes_pane/` directory whose `mod.rs` declares only, with siblings
by concern — header and summary, process rows, subagent rows, empty states.
Splitting first keeps the subagent work from being a 300-line append to a file
that is already over half the cap, and satisfies the repo's "`mod.rs` declares;
it does not implement" rule.

## Risks / Trade-offs

- **The recognizer is pinned to a raw-input shape the adapter can change.** →
  Fixtures captured from a real adapter run are the contract, as
  `knot-mcp-probe` does for a parsed CLI. Failure is closed: an unrecognized
  call yields no record, so the section reads "dispatched none" — the
  pre-change behavior, not a new wrong answer. A record is never invented from
  a partial match.
- **Terminal-mode Claude shows nothing until a plugin outside this repo
  emits.** → Handled by shipping the column as `ToolCalls`, so the section says
  *unavailable* rather than *none*. The user is told Knot cannot tell, which
  is accurate and self-explaining.
- **A user reads the two groups as one nested tree and assumes the `rg` below
  belongs to the `discovery` subagent above.** → The spec requires two labelled
  groups, not indentation, precisely so the adjacency reads as adjacency. No
  row is ever indented under another.
- **Registry growth inside a single long turn.** → The per-agent cap above.
  Running records are exempt, so the cap can never hide live work.
- **Fifteen conditions in `repaint_poll_tick`'s `if` chain becomes sixteen.** →
  Accepted. The chain is the documented place a new off-thread source reaches a
  frame, and the alternative — notifying unconditionally — repaints at the poll
  rate.
- **`ToolCallStart` and `ToolCallUpdate` gain a field.** → Additive and
  optional; every existing construction site and match arm keeps compiling with
  a default, and the ACP delta requires an absent raw input to stay
  distinguishable from an empty one.

## Migration Plan

None required. Every change is additive: a new crate, an optional field on two
protocol variants, a new column with a defined value for every existing roster
row, a new hook event on an existing route, and a new group in an existing
section. No persisted data, no settings key, and nothing to roll back beyond
reverting the change.

## Open Questions

- **Do the Codex, Gemini and OpenCode ACP adapters expose a delegation in
  their tool-call raw input, and in what shape?** Safely deferrable: the
  recognizer registry is per-type, and a type with no recognizer already has a
  defined state (`SubagentReporting::None` → *unavailable*). Answering it adds
  a recognizer and flips one column value; it changes no spec, no structure and
  no other task. Each such type is its own follow-up issue, the way
  `import-subagents-from-more-tools` splits one reader per issue.
