## ADDED Requirements

### Requirement: ACP launch path
When an agent's type has a registered ACP adapter and the agent is in Panel
view mode, the system SHALL launch that adapter as a subprocess (the
adapter's declared command plus the agent's working directory) and connect
to it as an ACP client, instead of assembling a terminal shell command for
that agent. Terminal mode and agent types with no registered adapter SHALL
continue to use the existing terminal launch command exactly as today.

#### Scenario: Panel-mode agent with no adapter
- **WHEN** an agent is in Panel view mode but its agent type has no
  registered ACP adapter
- **THEN** the system SHALL fall back to the existing terminal launch command
  for that agent rather than failing to start

#### Scenario: Switching an agent to Panel mode after launch
- **WHEN** the user switches an already-running Terminal-mode agent to Panel
  mode
- **THEN** the system SHALL start a new ACP connection for that agent without
  restarting or otherwise disturbing its existing terminal process

### Requirement: Adapter-carried MCP and registration
For an agent launched via its ACP adapter, MCP server configuration and
knot's inline registration arguments (per `agent-launch-command`'s existing
per-agent-type rules) SHALL be passed to the adapter subprocess through its
own configuration mechanism (arguments or ACP session configuration),
producing the same effective MCP/registration behavior as the terminal
launch path for that agent type.

#### Scenario: MCP disabled
- **WHEN** the MCP server is disabled
- **THEN** an ACP-launched agent SHALL start with no MCP configuration and no
  registration arguments, matching the terminal launch path's behavior when
  MCP is disabled
