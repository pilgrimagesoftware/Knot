## MODIFIED Requirements

### Requirement: Base command and user options

For a shell agent, the system SHALL start from the agent's configured custom
shell command, if any, and otherwise produce an empty agent command (a plain
interactive shell). This builder no longer applies to non-shell agent types,
which launch exclusively through the ACP adapter path.

#### Scenario: Missing command yields nothing

- **WHEN** a shell agent has no custom command configured
- **THEN** the built agent command is empty

## REMOVED Requirements

### Requirement: Initialization wrapper
**Reason**: The `KNOT_AGENT_ID`-prefixed half of this requirement only
applied to non-shell agents, which no longer launch through this builder.
**Migration**: See the new "Initialization wrapper" requirement below, scoped
to shell agents only.

### Requirement: Resume and fork arguments
**Reason**: These CLI resume/fork flags (`--resume`, `codex resume`/`fork`,
etc.) were only ever assembled for non-shell agent types building a terminal
command. Non-shell agents now launch exclusively through the ACP adapter
path, which negotiates resume through `session/load` (see `agent-lifecycle`),
never through this builder.
**Migration**: No action needed; ACP resume behavior is unchanged by this
removal.

### Requirement: MCP configuration injection
**Reason**: This CLI-argument-string approach (`--mcp-config`,
`--allowed-tools`, etc.) was reused, unmodified, as the ACP adapter path's
`AdapterLaunch.mcp_config` — but that path hands the whole string to the
subprocess as a single unparsed `argv` element, so it never actually
configured MCP access for any ACP-launched agent. Shell agents never needed
MCP. There is no remaining use for this requirement.
**Migration**: See the new "MCP configuration for ACP-launched agents"
requirement below.

### Requirement: Inline registration arguments
**Reason**: Terminal-command-specific; non-shell agents now launch
exclusively via ACP, which already registers agents through the ACP protocol
prompt (`acp_registration_prompt`), independent of this builder.
**Migration**: No action needed; ACP registration is unaffected.

### Requirement: Persona injection
**Reason**: Terminal-command-specific (claude/codex only); the ACP
registration prompt already carries persona text independently of this
builder.
**Migration**: No action needed; ACP persona injection is unaffected.

### Requirement: Registration arguments on resume or fork
**Reason**: Terminal-command-specific; the ACP resume path's registration
behavior is governed by `acp_registration_prompt`, not this builder.
**Migration**: No action needed; ACP resume/fork registration is unaffected.

## ADDED Requirements

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
