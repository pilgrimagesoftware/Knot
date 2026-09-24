# agent-launch-command Specification

## Purpose
Defines how the shell command that launches an agent in its terminal is
assembled from settings and per-agent-type rules: the base command and user
options, resume/fork arguments, MCP configuration and hook plugin injection,
inline registration arguments, the working-directory wrapper, the
`KNOT_AGENT_ID` environment variable, leading-space history suppression, the
shell-agent path, and what the knot instructions in a registration prompt have
to say.

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

The adapter's declared command SHALL be resolved against a search path made
of the process's own `PATH` merged with the standard non-sandbox install
locations (`/opt/homebrew/bin`, `/usr/local/bin`, `~/.cargo/bin`,
`~/.local/bin`, `~/.npm-global/bin`), with the install locations appended
after the process `PATH` entries and only where the process `PATH` does not
already name them, so a registered adapter installed in one of those
locations SHALL launch even when the app was not launched from a shell.
`~`-prefixed locations SHALL be resolved against the `HOME` environment
variable. Install commands run for an adapter that declares an install
method SHALL run under the same merged search path.

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

#### Scenario: Adapter installed outside the launchd PATH
- **WHEN** the app was launched from Finder (so its `PATH` is
  `/usr/bin:/bin:/usr/sbin:/sbin`) and the agent's adapter is installed in
  `/opt/homebrew/bin` or another standard install location
- **THEN** the adapter subprocess SHALL start successfully instead of failing
  with a "not found" spawn error

#### Scenario: Adapter install command uses the merged path
- **WHEN** an adapter's binary is not found on the merged search path but the
  adapter declares an install method
- **THEN** the install command runs with the same merged search path, so a
  package manager itself installed in a standard location is found

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

### Requirement: Knot instructions given to a launched agent

Every registration prompt the system sends an agent - the ACP protocol prompt
on a fresh session, and the deferred shell-agent prompt - SHALL carry a single
block of knot instructions, and that block SHALL meet all of the following.

It SHALL state that the agent's current task outranks a request from a
teammate, and that such a request is queued work rather than an interrupt. An
instruction that tells an agent to take on what a teammate asks without saying
what becomes of the work in hand is what let seven agents preempt one another
into a standstill; the ordering is the fix, so it is the requirement.

It SHALL decide handoff by folder rather than by project, and SHALL say
explicitly that a folder the agent shares with a teammate is the agent's to
work in. A project-ownership test is undecidable in the workspace Knot is
built for - one project, many agents - because every agent is in that project.

It SHALL name the conditions under which the agent replies to a teammate, and
SHALL forbid opening a design debate, seeking consensus, and waiting for
approval before acting. Without a bound, an answered message is an invitation
to another one.

It SHALL continue to carry the agent's knot agent ID verbatim, the name of the
MCP server the knot tools come from together with the reason that name
matters, the names `list-agents`, `send-message`, `broadcast-message` and
`set-status` each at the moment it is to be used, and the requirement to call
`set-status` before starting, on changing direction, and on finishing.

It SHALL be a single line. The shell-agent path types the prompt into a
terminal and then sends Return, so an embedded newline submits it
half-written.

It SHALL render in at most 1,000 characters. Every launched agent pays for
this text in its context window on every launch, so an instruction added here
is funded by compressing wording elsewhere, not by raising the ceiling.

#### Scenario: A teammate's request does not preempt the agent

- **WHEN** an agent is launched and reads its knot instructions
- **THEN** those instructions tell it to finish its current task before taking
  up a teammate's request, and describe that request as queued work rather
  than an interrupt

#### Scenario: A shared folder is the agent's to work in

- **WHEN** several agents are launched against the same folder
- **THEN** the knot instructions tell each of them that a folder shared with a
  teammate is theirs to work in, and reserve handing work over for a folder
  that is only the teammate's

#### Scenario: The reply loop is bounded

- **WHEN** an agent has been asked something by a teammate
- **THEN** its knot instructions name the conditions for replying and forbid
  opening a design debate, seeking consensus, or waiting for approval before
  acting

#### Scenario: The instructions stay within budget

- **WHEN** the knot instructions are rendered for any agent ID
- **THEN** the result is one line of at most 1,000 characters
