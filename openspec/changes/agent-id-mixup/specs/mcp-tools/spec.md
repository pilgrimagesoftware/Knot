# Spec Delta

## MODIFIED Requirements

### Requirement: register-agent

`register-agent` SHALL require `agentId` and accept optional `sessionId`. On
success it SHALL mark the agent registered, associate the session id when
given, and return the unread-message count and the list of knot members
visible to the caller.

It SHALL fail with an error the agent sees, and change nothing, when `agentId`
names an agent another live connection holds, or when the calling connection
is bound to a different agent (see "A caller acts only as itself"). A
successful registration from a connection bound to no agent SHALL bind it to
the registered agent.

#### Scenario: Register returns roster

- **WHEN** a known agent calls `register-agent` with its id
- **THEN** it is marked registered and the result includes the current knot
  members

#### Scenario: A duplicate live registration is refused

- **WHEN** one live connection holds an agent and a second session calls
  `register-agent` with that agent's id
- **THEN** the second call fails with an error saying another live session
  holds the agent, and the first connection keeps it

## ADDED Requirements

### Requirement: A caller acts only as itself

Every tool names its caller in an argument: `from` for `send-message` and
`broadcast-message`, `agentId` for every other tool. The server SHALL bind an
MCP connection to the agent its URL names in an `agent` query parameter
(`…/mcp?agent=<id>`), which Knot SHALL put in the URL it gives each agent it
launches; a new connection for an agent SHALL replace that agent's previous
one. A call whose caller argument does not resolve to the connection's bound
agent SHALL fail with an error that names the bound agent and its id, and
SHALL change nothing. On a connection bound to no agent, a call whose caller
argument names an agent another live connection holds SHALL fail the same way.
A call naming no caller, or naming an agent Knot does not know on a connection
bound to none, SHALL be left to the tool.

#### Scenario: A connection naming another agent is refused

- **WHEN** a connection opened on Builder 1's URL calls `set-status` with the
  orchestrator's id
- **THEN** the call fails, the orchestrator's status is unchanged, and the
  error gives Builder 1's id

#### Scenario: A restarted agent takes over its connection

- **WHEN** an agent is restarted and its new session connects on its URL
- **THEN** the new connection holds the agent, and its registration succeeds
