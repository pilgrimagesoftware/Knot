# Spec Delta

## ADDED Requirements

### Requirement: A bound connection registers its agent

A request on an MCP connection whose URL names an agent (`…/mcp?agent=<id>`,
see `agent-id-mixup`'s "A caller acts only as itself") SHALL mark that agent
registered, when Knot knows it and it is not registered already. That happens
with no tool call and no model turn, and on every such request, so an agent
the registry did not yet hold when it connected is registered as soon as the
registry does. Nothing SHALL be persisted, since registration is runtime state.
`register-agent` SHALL go on registering an agent from a connection whose URL
names none, and SHALL keep refusing an agent another live connection holds.

#### Scenario: A resumed agent is registered by its connection

- **WHEN** Knot restarts and an agent's resumed session connects on its own URL
- **THEN** `list-agents` shows it registered before it makes any tool call

#### Scenario: An unbound connection registers nobody by connecting

- **WHEN** a connection whose URL names no agent initializes
- **THEN** no agent's registration changes
