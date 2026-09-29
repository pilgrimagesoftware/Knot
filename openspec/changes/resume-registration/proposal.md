# Proposal

## Why

After a Knot restart, every agent whose session really resumed shows
`isRegistered: false` (#552). Since #545, `connect_into` sends the registration
turn only to a fresh session, and registration lives in the server's memory, so
a resumed agent is connected, and since #544 bound to its own ID, but never
registered. An agent whose first registration turn failed ("Not logged in")
never gets another chance. The orchestrator picks agents from that registry, so
after every restart it sees nobody.

## What Changes

- A request on a connection whose URL names an agent registers that agent on the
  server, with no tool call, no model turn and nothing in the conversation.
- For adapters not known to keep the URL's query (everything but Claude and
  Codex), a resumed session is sent a one-line registration request.
- `register-agent` and its duplicate-live-holder refusal are unchanged.

## Capabilities

### Modified Capabilities

- `mcp-tools`: adds "A bound connection registers its agent".
- `agent-lifecycle`: adds "A resumed agent is registered".

## Impact

- `knot-mcp`: `ToolCatalog::connected`, called for every request on a bound URL.
- `knot-mcp-tools`: `McpToolCatalog::connected` marks the agent registered.
- `knot-agent-launch`: `keeps_mcp_query`, `resume_registration_prompt`, consts.
- `knot`: `ConnectRequest.resume_registration_prompt`, sent only on a real resume.
