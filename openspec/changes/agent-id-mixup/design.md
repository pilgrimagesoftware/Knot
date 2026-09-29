# Design

## Context

The builder's transcript began with a first turn that carried the orchestrator's
ID. At that point `list-agents` showed the orchestrator unregistered and the
builder idle, and the builder registered the orchestrator's ID itself. Of the
paths that could put another agent's ID in front of a session, only the restore
fallback moves a conversation between agents: fork copies to a new agent on
purpose, duplicate and restart copy nothing, and `from_saved` matches by ID.
`resolve_resume_sessions` fell back to `load_sessions(folder).first()` for any
agent without a persisted session, whoever else shared the folder.

## Decisions

### One session, one agent

Sessions already claimed are collected first: any existing resume ID, then the
persisted ones in roster order, where a session two records name goes to the
earlier agent. The history fallback then runs only for an agent alone with its
type in its folder, and skips claimed sessions. When the history can't say whose
a session is, the agent starts fresh rather than guessing. Starting fresh loses
nothing that belonged to it.

### Bind the connection, not the argument

The tools can't tell who is calling from `agentId`, since that argument is the
thing that went wrong. Knot knows which agent it launched, so the URL it hands
out carries the ID (`?agent=<id>`). Checked live: `claude-agent-acp` sends the
query on every MCP request. The server binds the MCP session to that agent, and
the tool catalog refuses any call whose `from`/`agentId` resolves to a different
agent. The refusal names the right ID, so a confused agent corrects itself on its
first call instead of hours later.

A new `initialize` for an agent replaces its previous session, as the session
manager already did. So a restarted agent takes its binding over, and liveness
never has to be judged by timeout.

### Registration for unbound connections

A client without a Knot URL can still register, but not as an agent another
live connection holds. Success binds its session, if it keeps one, so a second
duplicate is caught as well. A client that opens a fresh session on every
request is never bound: holding one would lock out the next request.

### `call_as` beside `call`

`ToolCatalog::call_as` defaults to `call`, so existing catalogs and tests are
unchanged. The server dispatches through `call_as` with a `Caller` (bound agent,
kept session, session manager). `McpToolCatalog` checks identity there, then
calls `call`.

## Risks / Trade-offs

- [Two agents sharing a folder and type no longer auto-resume from history] ->
  They resume from their own persisted session IDs, which Panel agents record
  when a session opens; the history fallback was only ever a guess for them.
- [A client drops the URL's query] -> The connection is unbound and the
  registration check still applies; the adapter check covers `claude-agent-acp`.
