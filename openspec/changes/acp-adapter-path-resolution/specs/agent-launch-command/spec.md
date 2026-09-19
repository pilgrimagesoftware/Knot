# agent-launch-command Delta Spec

## MODIFIED Requirements

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