## Why

Every agent today runs behind a raw terminal grid (`knot-terminal` + `terminal_view.rs`):
readable, but it throws away structure the agent already emits — message
boundaries, tool calls, diffs, permission prompts — and re-derives coarse
state (Idle/Working/Awaiting input/Error) from ANSI heuristics and hooks. The
Agent Client Protocol (ACP) is an open, JSON-RPC-based standard (see
agentclientprotocol.com) that several coding agents already speak or can
speak through a thin adapter, and it carries that structure directly: typed
session updates, tool-call metadata, diffs, and permission requests instead
of a character grid. Zed's agent panel is built on it. Adopting ACP for the
agents that support it lets Knot render a native, information-dense panel
(à la Zed) instead of a terminal transcript, while the terminal stays
available — unchanged — for actual shell use and for any agent type ACP
doesn't cover.

## What Changes

- Add an ACP client (`knot-acp`): JSON-RPC 2.0 over stdio to a subprocess,
  `initialize` capability negotiation, `session/new` (or `session/load` for
  resume), `session/prompt`, streaming `session/update` notifications,
  `session/request_permission`, and `session/cancel`. Agent-agnostic; no
  agent-specific behavior lives here.
- Add a per-agent-type ACP adapter registry describing, for each supported
  agent type, whether it speaks ACP natively or needs a wrapper subprocess
  (e.g. `claude-code-acp`), the command to launch that wrapper, and its
  declared capabilities (resume support, permission modes, MCP passthrough).
  Agent types with no known ACP path keep launching straight into the
  terminal exactly as today — **not a regression, a fallback**.
- Add a chat-style agent panel UI in `crates/knot`: streaming assistant text,
  tool-call cards (command/diff/result), inline permission-request prompts
  with allow/deny, and a thought/reasoning collapse — replacing the terminal
  grid as the *default* view for ACP-capable agents.
- Add a per-agent view-mode toggle (Panel / Terminal). The terminal stays
  fully functional and becomes the explicit way to get a real shell — it is
  never removed, only demoted from "the only view."
- Wire agent state (`AgentState`: Idle/Working/Awaiting input/Error) from ACP
  session updates and permission requests for ACP-managed agents, in place
  of the terminal-output/hook heuristics `activity-detection` currently uses
  for those agents. Hook-based detection remains the path for terminal-mode
  and non-ACP agents.
- Session id / resume plumbing (`agent-lifecycle`, `conversation-history`)
  gains an ACP session id alongside the existing terminal-resume session id,
  since ACP's `session/load` and a CLI's own `--resume <id>` are not always
  the same identifier space.

**Non-goals for this change:**
- Remote (HTTP/WebSocket) ACP transport — stdio subprocess only.
- ACP protocol v2 (session fork, per-session MCP attachment) — v1 only.
- Confirming exact native-vs-adapter ACP status for every target agent is
  implementation-time work (see design.md); this proposal scopes the
  mechanism, not a locked-in list.
- Multi-agent (sub-agent) sessions, terminal-inside-panel embedding, or
  slash-command catalogs — panel parity with Zed's *full* feature set is out
  of scope; this covers prompt/response, tool calls, diffs, and permissions.

## Capabilities

### New Capabilities
- `acp-client`: transport, JSON-RPC framing, capability negotiation, session
  lifecycle (new/load/prompt/update/cancel/close), and permission-request
  handling for talking to an ACP-speaking agent subprocess.
- `acp-panel-ui`: the GPUI panel that renders an ACP session as a chat-like
  view (streaming text, tool calls, diffs, permission prompts) and the
  per-agent Panel/Terminal view-mode toggle.

### Modified Capabilities
- `agent-launch-command`: agent types with a registered ACP adapter launch
  through that adapter (subprocess + stdio, not a terminal shell command)
  when the agent is in Panel mode; Terminal mode and unsupported agent types
  are unchanged.
- `agent-lifecycle`: adds an ACP session id field alongside the existing
  session id/resume-session id, and defines how it's set on create/resume/
  restart for ACP-managed agents.
- `activity-detection`: for ACP-managed agents, `AgentState` is derived from
  ACP session updates (turn start/end, permission requests) instead of
  terminal-output/hook heuristics; the existing rules are unchanged for
  terminal-mode and non-ACP agents.

## Impact

- New crate `knot-acp` (protocol client), depending only on `knot-core` +
  `serde`/`serde_json` + `tokio` (process spawn, stdio framing).
- `crates/knot-agent-launch`: new adapter-registry module; `command.rs`
  branches on view mode / adapter availability.
- `crates/knot-agents`: `Agent` gains an ACP session id field and mode flag;
  `store.rs` routes state updates from either the terminal/hook path or the
  ACP path depending on mode.
- `crates/knot`: new panel view module (parallel to `terminal_view.rs`),
  view-mode toggle in the agent header, permission-prompt UI, wiring into
  the existing sidebar/content layout.
- `crates/knot-activity`: gains an ACP-update-driven state transition path
  alongside the existing terminal/hook-driven one.
- No change to `knot-terminal`, `knot-git`, `knot-discovery`, `knot-mcp*`,
  `knot-history`, or `knot-watch`.
