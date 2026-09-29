# Proposal

## Why

An agent was launched on another agent's knot ID and acted as it for hours
without anything noticing (#539). The builder read the orchestrator's inbox,
overwrote its status and registered as it. The builder's own ID stayed
unregistered and its messages unread, so a dispatched task sat until it was
reassigned.

Two things let that happen:

- **Launch.** With restore-conversation-on-launch on, an agent without a
  session of its own fell back to the newest conversation in its CLI's history
  for its folder and type. The history is kept per folder, not per agent, so an
  agent sharing a folder with another of its type could be handed that agent's
  conversation. The transcript carried the other agent's ID, and a resumed
  session gets no registration prompt to correct it.
- **Server.** The knot MCP tools took any known ID from any session. Two live
  sessions could register as one agent and share its inbox and status in
  silence.

## What Changes

- The restore fallback gives a session to one agent at most. It is not used for
  an agent that shares a folder and type with another, and it never hands out a
  session another agent's record names.
- Each launched agent's MCP URL names its ID (`…/mcp?agent=<id>`), and the
  server binds the connection to that agent. A call whose `agentId`/`from` names
  a different agent is refused with the right ID.
- `register-agent` on an ID another live connection holds is refused with an
  error the agent sees. A successful registration binds an unbound connection.

## Non-Goals

- Changing what the knot instructions say. #536 already carries them through the
  system channel on every launch, resumes included.
- Identifying clients that don't use a Knot-issued URL beyond the registration
  check.

## Capabilities

### Modified Capabilities

- `agent-lifecycle`: the restore fallback's one-agent rule.
- `agent-launch-command`: the ID in the instructions and the URL is the agent's own.
- `mcp-tools`: `register-agent` refuses a duplicate live registration; adds "A caller acts only as itself".

## Impact

- `knot-agents`: `AgentStore::resolve_resume_sessions`.
- `knot-agent-launch`: `agent_mcp_url`; `knot-core`: `MCP_AGENT_QUERY`.
- `knot-mcp`: session binding (`bind`, `held_elsewhere`), `Caller`,
  `ToolCatalog::call_as`, the `agent` query parameter.
- `knot-mcp-tools`: the `identity` refusal policy in `call_as`.
- `knot`: panel sessions connect on the agent's own URL.
