# Proposal

## Why

An agent working on a real task delegates. Claude Code spawns a discovery
agent with its `Task` tool, waits on it, and reports back; Codex and Gemini
have their own delegation. From Knot the delegation is invisible. The agent
pane's processes section — the one place that answers "what is this agent
actually doing right now" — lists operating-system processes only: `node`,
`rg`, a `cargo` build. A subagent is not one of those. Claude Code runs it
inside the same process as the parent, so `ps` has nothing to report, and
`openspec/specs/agent-processes/spec.md` is a process-tree contract from end
to end (`crates/knot-processes/src/table.rs:59` — `DescendantProcess { pid,
ppid, command, elapsed, activity }`).

So a user watching an agent that has been quiet for four minutes cannot tell
whether it is stuck, compiling, or waiting on three subagents it dispatched.
The section shows `node +2 more` either way. The thing the user most wants
named — the agent's own delegated work — is the one thing the section cannot
name, because Knot has no runtime representation of a subagent anywhere in
the workspace: `SessionUpdate` has no such variant
(`crates/knot-acp/src/protocol/mod.rs:163`), `ToolCallCard` keeps `kind` as
an opaque passthrough string (`crates/knot/src/panel_state/message.rs:19`),
and `agent_type::ALL` models no parent/child relation
(`crates/knot-core/src/agent_type.rs:44`).

The existing `import-subagents-from-more-tools` change is unrelated: it reads
persona *definition* files off disk (`~/.claude/agents/*.md`) to populate
Knot's personas. It concerns what a subagent could be, never what one is
doing.

## What Changes

- **The processes section lists two kinds of entry, in two labelled groups.**
  Subagents first — they are the agent's own account of its work — then the
  operating-system processes the section already lists, unchanged. One
  collapsible section, as today; the groups are how the two kinds stay told
  apart rather than blended into one list where a row's meaning depends on
  reading its fields.
- **A subagent row says what the agent said about it**: the kind of subagent
  it dispatched, the task it was given, how long it has been running, and
  whether it is running, finished, or failed. No PID, because it has none.
- **Subagent rows offer no terminate action.** A subagent is not a process
  Knot can signal; it lives inside the agent's own process, and the only way
  to stop one is to interrupt the agent. Offering `Terminate` on a row where
  it would have to mean "interrupt the whole agent" is worse than not
  offering it. The row keeps the copy actions, carrying the task text out.
- **Two feeds, one model, named per agent type.**
  - *Panel (ACP) agents*: the tool-call stream already arriving. ACP carries
    a tool call's `rawInput`, which Knot currently discards; parsing it is
    what lets a delegation tool call be recognized as one and its subagent
    kind and task read off. `kind` and `title` alone cannot do it — `kind`
    is an icon hint whose delegation value is adapter-specific, and `title`
    is prose.
  - *Terminal (PTY) agents*: the hook route Knot already serves
    (`POST /api/v1/agent/status`, `crates/knot-mcp/src/hooks.rs:12`), which
    already takes an arbitrary `hook` name and `payload`. Claude Code's
    `PreToolUse`/`PostToolUse`/`SubagentStop` events carry exactly the
    delegation facts, at the moment they happen.
- **A type that cannot report says so, and it is not "none".** An agent type
  with no recognizer, or a Claude agent whose hook plugin predates the
  subagent events, reads as *subagent reporting unavailable*, never as *no
  subagents*. The distinction is the whole value of the row: "it dispatched
  nothing" and "Knot cannot tell" are opposite answers to the user's
  question.
- **The collapsed header names subagents before processes.** A header with
  the room for three names spends them on `discovery, code-review` before
  `node`, because the delegation is the more informative fact and the one
  the user cannot get anywhere else in Knot.
- **Subagent state survives a collapse and a re-render, and is not sampled.**
  Unlike the process list, which is a fresh `ps` read every interval,
  subagent state is a log of events that arrived: a subagent that started
  and finished is finished, and the record of it persists until the agent's
  turn ends rather than vanishing between two samples.

### What this deliberately does not try to be

Knot cannot attribute an operating-system process to a subagent. When a
Claude Code subagent runs `rg`, that `rg` descends from the same session root
as everything else the agent runs, and `ps` reports no evidence of which
context spawned it. The two groups are therefore adjacent, not nested: the
section never claims a process belongs to a subagent, because it cannot know
and a wrong nesting is worse than none.

Nor is this a second transcript. A subagent's own output, its tool calls, and
its conversation stay where they already are — the panel's tool-call cards,
or the agent's terminal. The section reports that a subagent exists, what it
was asked for, and how long it has been at it. That is the question the
processes section answers for processes, asked of the other kind of work.

### Cost

None on the render path, and no new polling. The ACP feed is a branch in the
fold that already runs per `session/update`; the hook feed is a branch in a
handler that already runs per hook POST. Both write state the existing
`repaint_poll_tick` chain drains, the way published process samples already
reach a frame. No new subprocess, no new timer, no new file read.

### Non-goals

- **Terminating or interrupting a subagent.** Knot has no mechanism, and
  neither ACP nor the hook protocol offers one.
- **Attributing processes to subagents.** Named above; unknowable from `ps`.
- **Nesting subagents within subagents.** A subagent that delegates further
  is reported flat, at one level. Claude Code does not currently permit it,
  and modelling a tree for a case that cannot arise buys a shape nothing
  fills.
- **Parsing the agent's transcript file.** Knot already holds
  `transcript_path` and reads it for the last assistant message
  (`crates/knot-mcp/src/hooks.rs:108`), so scraping subagent invocations out
  of the JSONL is available and is being declined: it pins Knot to an
  undocumented format that changes under it, to learn something the hook
  feed reports directly and on time.
- **Shipping the hook emitter.** The Claude plugin that POSTs to Knot's hook
  route lives outside this repo. This change defines the event Knot accepts
  and degrades honestly until something sends it; wiring the plugin is
  separate work, tracked as its own issue.
- **A subagent marker on the sidebar agent row or the dashboard.** Worth
  having, and separate changes against `agent-list-ui` and `dashboard`.
- **Knot's own MCP-created agents.** `create-agent` and `dispatch-task` make
  top-level Knot agents with their own panes and their own processes
  sections. They are not subagents and do not appear in this list.

## Capabilities

### New Capabilities

- `agent-subagents`: what a subagent is to Knot — its typed model and
  lifecycle, the two feeds that produce it, which agent types can report one
  and which cannot, and how an unreportable type is distinguished from an
  agent that dispatched nothing.

### Modified Capabilities

- `agent-processes`: the section gains a second group. The requirement for
  what the pane presents changes (two labelled groups, subagent rows without
  a PID or terminate action), as do the collapsed header summary, the empty
  states, and the requirement that all section text is localized.
- `acp-client`: the streaming-session-updates requirement changes — a tool
  call's `rawInput` is carried through rather than discarded, so a delegation
  call can be recognized.
- `agent-hooks`: the status route accepts subagent lifecycle events, with a
  defined payload shape and the rejection behavior for a malformed one.

## Impact

- **New crate `knot-subagents`** — the typed model (`Subagent`,
  `SubagentState`, `SubagentId`) and one recognizer per agent type, turning a
  tool call's raw input or a hook payload into a lifecycle event. Subprocess-
  free, GPUI-free and unit-testable on fixtures, the way `knot-git` and
  `knot-forge` are. Named apart from `knot-core::import::subagents`, which
  reads definition files and is unrelated.
- **`crates/knot-acp`** — `ToolCallStart` and `ToolCallUpdate` gain the raw
  input ACP already sends and Knot drops
  (`crates/knot-acp/src/protocol/mod.rs:169`). Additive; every existing
  variant keeps its fields.
- **`crates/knot/src/panel_state/`** — `ToolCallCard` carries the raw input
  through `fold.rs`, and the fold feeds the recognizer. `summary.rs`'s
  `ToolRunSummary` is untouched; a delegation call keeps counting as a call.
- **`crates/knot-mcp/src/hooks.rs`** — a third handler alongside `register`
  and `status` on `AgentHookHandler`, or a branch within `status` keyed on
  `hook`. Either way the route, the `agent_id` validation and the 400
  behavior are the existing ones.
- **`crates/knot-core/src/agent_type.rs`** — one column for how a type
  reports subagents (ACP raw input, hook events, or not at all), plus the
  roster's existing completeness test.
- **`crates/knot/src/agent_processes.rs` and
  `crates/knot/src/workspace_window/`** — `ProcessSection` gains the subagent
  list beside its process snapshot, a branch in `repaint_poll_tick` so
  subagent events reach a frame, and the two-group render.
  `render/processes_pane.rs` is 412 lines before this change and will need
  splitting by concern to stay under the 700-line cap.
- **`crates/knot-core/locales/en.yml`** — new keys under `processes.` for the
  group labels, the subagent row's fields and states, the unreportable case,
  and the new empty states.
