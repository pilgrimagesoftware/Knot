## Context

See `proposal.md` - Why. Relevant existing code:

- `crates/knot-terminal`: Ghostty-backed PTY + grid (`pty.rs`, `grid.rs`),
  runtime-agnostic (no async runtime baked in).
- `crates/knot-agent-launch`: assembles the terminal shell command per agent
  type (`command.rs`), including MCP args and inline registration
  (`registration.rs`), per the `agent-launch-command` spec.
- `crates/knot-agents`: `Agent` struct + `store.rs` (1132 lines) - the
  runtime agent list, state machine, and persistence-to-`SavedAgent`
  conversion (`convert.rs`).
- `crates/knot-activity`: the Idle/Working/Awaiting-input/Error state
  machine described by `activity-detection`, driven by terminal output,
  keystrokes, and hook events.
- `crates/knot/src/terminal_view.rs`: renders a `Grid` as GPUI elements - a
  cell-grid renderer, not a native surface (154 lines, see
  `terminal-rendering` design.md).
- Both `knot-terminal` and `knot-agent-launch` are runtime-agnostic; `knot`
  is the only crate currently depending on `tokio` for UI-driven async work.

**ACP itself** (agentclientprotocol.com; reference SDK
`agentclientprotocol/rust-sdk`, crate `agent-client-protocol`): JSON-RPC 2.0
over stdio to a subprocess. `initialize` negotiates protocol version and
capabilities; `session/new` or `session/load` opens a session;
`session/prompt` submits a turn; `session/update` notifications stream text
deltas, tool-call lifecycle, and diffs; `session/request_permission` is a
server-to-client request the client must answer before the turn continues;
`session/cancel` aborts. All paths in messages are absolute; line numbers are
1-based. Zed's implementation (`acp_thread`, `agent_servers`, `agent_ui`
crates in zed-industries/zed) follows this shape: one long-lived subprocess
per session, an `AgentConnection` trait abstracting native vs. adapter-wrapped
agents, and UI state built by folding the update stream (text accumulation,
tool-call cards, diff computation from consecutive snapshots, a permission
gate that blocks further prompts).

**Per-agent ACP status** (from research; verify against each agent's current
CLI docs before wiring an adapter, since this space moves fast and one
research pass isn't authoritative):

| Agent | Expected path |
|---|---|
| Claude Code | Adapter subprocess (`claude-code-acp`-style wrapper); does not speak ACP directly over its own CLI invocation |
| Codex | Adapter or native flag, unconfirmed - check `codex --help` / docs for an ACP or JSON-RPC mode |
| Gemini CLI | Reported to have native ACP support as of late 2025 - confirm flag/subcommand |
| OpenCode | Community ACP support reported - confirm current package name |
| Cursor | CLI docs mention ACP (`cursor.com/docs/cli/acp`) - confirm invocation |
| GitHub Copilot CLI | ACP support reported in preview - confirm availability/flag |
| QwenCode | Native ACP reported - confirm invocation |

Every row above is a hypothesis to validate in `tasks.md`'s spike task, not a
committed fact. The adapter registry (Decision 2) is designed precisely so a
wrong guess here costs one config entry, not a redesign.

## Goals / Non-Goals

**Goals:**
- A generic ACP client crate usable by any agent type without per-agent code
  in the protocol layer itself.
- A panel UI that reads naturally as "the agent's actual conversation," not
  a terminal skin.
- Zero regression for agent types without an adapter, and zero forced
  migration for users who prefer the terminal.

**Non-Goals (design-level, beyond proposal.md's non-goals):**
- Building or vendoring any agent's ACP adapter binary ourselves. Knot
  invokes an adapter command the user's environment already provides
  (installed via npm/cargo/the agent's own installer); Knot ships
  configuration (command + args), not the adapter.
- A shared "universal" tool-call renderer that tries to model every possible
  tool schema generically. Render by ACP's own typed tool-call `kind`
  (execute, read, edit, etc.); an unknown kind falls back to a generic
  input/output card.
- Persisting ACP conversation transcripts separately from what
  `conversation-history` already reads from each agent's own on-disk
  format - the panel replays from the live session or from `session/load`,
  it does not become a second history store.

## Decisions

**1. `knot-acp` as a standalone, runtime-agnostic-where-possible crate.**
Mirrors `knot-terminal`'s shape: owns transport + protocol types, exposes an
async API (subprocess I/O is inherently async, so this crate does take a
`tokio` dependency, unlike `knot-terminal`/`knot-agent-launch`) but has no
knowledge of `Agent`, GPUI, or agent-type-specific launch rules. Alternative
considered: fold ACP handling directly into `knot-agents`. Rejected - it
would couple protocol plumbing to the agent runtime model and make the
protocol layer untestable without the rest of the store.

**2. Adapter registry lives in `knot-agent-launch`, keyed by agent type.**
A small table (agent type -> adapter command template + capability flags:
`supports_resume`, `supports_permission_modes`) next to the existing
per-type command-building rules in `command.rs`. Alternative considered: a
user-editable settings table (like the agent-type command settings already
exposed). Rejected for v1 - the set of adapters is small and moves with
Knot releases, not per-user preference; revisit if users need to point at a
custom/local adapter build.

**3. `Agent` gains `view_mode: ViewMode` (Panel | Terminal) and
`acp_session_id: Option<String>`, not a replacement for the terminal
fields.** An agent in Panel mode still has an underlying terminal capability
(per proposal.md, the terminal is never removed) - it simply isn't the
active view or the active state-tracking source. Alternative considered:
make Panel/Terminal a property of the *view* only, with the terminal PTY
always running underneath. Rejected for v1 - running both a live PTY shell
*and* an ACP subprocess per agent doubles resource use and gives Panel mode
no real terminal to attach to anyway (the ACP adapter, not the shell, is the
running process); "open a terminal" for a Panel-mode agent instead spawns a
plain shell in that agent's folder on demand, same as it would for a fresh
shell agent.

**4. State machine gets a third tracking source (`acp-updates`) rather than
reusing the hook path.** Hooks are Claude/Codex-specific shell scripts keyed
to terminal escape sequences; ACP's `session/update` stream already carries
turn-start/turn-end/permission-request as typed events with no heuristics
needed. Reusing the hook plumbing would mean synthesizing fake hook payloads
from ACP events for no benefit. See the `activity-detection` delta spec.

**5. Panel UI is a new sibling to `terminal_view.rs`, not a mode inside
it.** `terminal_view.rs` renders a `Grid`; the panel renders a `Vec` of
session-update-derived message/tool-call/permission entries. Sharing a
render path would force one of the two to contort around the other's data
model. The agent header / view-mode toggle is the shared surface (per
`knot-ui-conventions.md`, styled consistently with existing header
conventions), not the content renderer.

## Risks / Trade-offs

- [Per-agent ACP support is a moving target and this research pass could be
  wrong for one or more agents] → the adapter registry (Decision 2) makes an
  incorrect assumption a one-entry config fix, not a spec change; agents
  default to terminal-only until their adapter is verified working in
  practice, per-agent, as an implementation task.
- [Adapters are external processes Knot doesn't control - a broken or
  missing adapter install is a plausible support burden] → Panel mode fails
  closed to Terminal mode with a visible error (per `acp-client`'s exit/error
  handling requirement), never a silent hang.
- [Two state-tracking paths (hook-based, ACP-based) now coexist in
  `knot-activity`] → they're mutually exclusive per agent via the tracking
  set (Decision 4), so this is added surface area, not added interaction
  complexity; existing hook-path tests are unaffected.
- [Protocol churn - ACP v2 is still draft] → this change targets v1 only
  (proposal.md non-goals); `knot-acp`'s version negotiation
  (`acp-client`'s capability-negotiation requirement) fails closed on a
  version mismatch rather than guessing at v2 fields.

## Migration Plan

No data migration - `view_mode` defaults to `Terminal` for all existing
agents on load (absent field -> default, consistent with how other runtime
fields already reset on load per `agent-lifecycle`). Rollout is purely
additive and can ship adapter-by-adapter: land `knot-acp` and the panel UI
behind the per-agent-type adapter registry, then enable one agent type at a
time as each adapter is verified, rather than a single flag-day cutover.

## Open Questions

- Exact ACP adapter invocation (binary name, install method, flags) for each
  target agent - resolved per-agent during implementation (tasks.md spike),
  doesn't change the specs or this design.
- Whether any target agent's adapter needs Knot to proxy its own MCP tools
  through ACP's session-scoped MCP attachment vs. the existing HTTP MCP
  server - resolved per-adapter, doesn't change the `acp-client` contract.
