# agent-launch-command Specification

## Purpose
Defines how the shell command that launches an agent in its terminal is
assembled from settings and per-agent-type rules: the base command and user
options, resume/fork arguments, MCP configuration and hook plugin injection,
inline registration arguments, the working-directory wrapper, the
`KNOT_AGENT_ID` environment variable, leading-space history suppression, and
the shell-agent path.

## Requirements

### Requirement: Base command and user options

For a shell agent, the system SHALL start from the agent's configured custom
shell command, if any, and otherwise produce an empty agent command (a plain
interactive shell). This builder no longer applies to non-shell agent types,
which launch exclusively through the ACP adapter path.

#### Scenario: Missing command yields nothing

- **WHEN** a shell agent has no custom command configured
- **THEN** the built agent command is empty

### Requirement: Shell agent command

For a shell agent, the agent command SHALL be the agent's custom shell command
when set, otherwise empty (a plain interactive shell).

#### Scenario: Custom shell command

- **WHEN** a shell agent has a custom command `htop`
- **THEN** the built agent command is `htop`

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

### Requirement: Shell agent initialization wrapper

The final terminal command SHALL be `<space>cd '<folder>' && clear` (a shell
agent's custom command, if any, appended after `clear && `). The leading
space suppresses shell history (given `ignorespace` / zsh default). Since
this path now only launches shell agents, no `KNOT_AGENT_ID` environment
variable is set (shell agents do not register with MCP).

#### Scenario: Shell agent wrapper with no custom command

- **WHEN** a shell agent with no custom command is launched
- **THEN** the command is `cd '<folder>' && clear` (leading space) and sets no
  `KNOT_AGENT_ID`

#### Scenario: Shell agent wrapper with a custom command

- **WHEN** a shell agent has a custom command `htop`
- **THEN** the command is `cd '<folder>' && clear && htop` (leading space)

### Requirement: MCP configuration for ACP-launched agents

The system SHALL configure MCP access for every ACP-launched (non-shell)
agent through the ACP protocol's own session-scoped mechanism (the
`session/new`/`session/load` request's MCP server parameters), naming the
knot HTTP MCP server and its URL, rather than through a CLI argument string
passed to the adapter subprocess. This SHALL apply uniformly across every
agent type with a registered ACP adapter, replacing the previous
per-type CLI-flag behavior, which never reached the subprocess correctly.

#### Scenario: ACP session carries MCP server config

- **WHEN** a non-shell agent with a registered ACP adapter is launched and
  the MCP server is enabled
- **THEN** its `session/new` (or `session/load`) request includes the knot
  MCP server's URL, and the agent can successfully call knot tools (e.g.
  `set-status`, `register-agent`) once the session starts

#### Scenario: MCP disabled omits ACP MCP config

- **WHEN** the MCP server is disabled
- **THEN** the ACP session request includes no MCP server configuration
