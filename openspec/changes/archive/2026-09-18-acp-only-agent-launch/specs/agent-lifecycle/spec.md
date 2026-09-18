## ADDED Requirements

### Requirement: View mode is fixed by agent type

Non-shell agent types (`claude`, `codex`, `opencode`, `gemini`, `copilot`)
SHALL always launch through the Panel (ACP) path; Terminal mode SHALL NOT be
offered or selectable for them. Shell agents (always companions, per the
shell-companion requirement) SHALL always launch through the Terminal (PTY)
path; Panel mode SHALL NOT apply to them, since a bare shell has no ACP
session to connect to.

#### Scenario: Non-shell agent has no Terminal mode

- **WHEN** a `claude`, `codex`, `opencode`, `gemini`, or `copilot` agent is
  created or restarted
- **THEN** it launches through the Panel/ACP path and no Terminal/Panel
  toggle is presented for it

#### Scenario: Shell companion stays in Terminal mode

- **WHEN** a shell companion agent is created or restarted
- **THEN** it launches through its PTY terminal session, never through the
  ACP/Panel path
