## ADDED Requirements

### Requirement: Shell agent removal on process exit

When a shell agent's terminal process exits, the system SHALL remove that
agent, following the same removal sequence as a user-initiated removal
(companions first, MCP unregister if registered, session teardown, then
removal from every workspace and the master list, re-selecting or
collapsing split panes). This removal SHALL NOT prompt for confirmation:
the process has already exited and there is nothing left to cancel.

This trigger applies only to agents whose agent type is `shell` - a shell
companion or a standalone shell agent. A non-shell agent has no terminal
process of its own and is unaffected.

#### Scenario: A shell companion's shell exits

- **WHEN** a shell companion's process exits (the user types `exit`, or the
  shell dies)
- **THEN** the companion is removed without a confirmation prompt, and the
  split pane that showed it is collapsed

#### Scenario: A standalone shell agent's shell exits

- **WHEN** a shell agent that owns no companions and is owned by none has
  its process exit
- **THEN** that agent is removed without a confirmation prompt

#### Scenario: A non-shell agent is unaffected

- **WHEN** an ACP-launched agent's adapter subprocess exits
- **THEN** the agent is not removed; its panel reports the ended session
  per `acp-panel-ui`
