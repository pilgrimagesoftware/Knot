# Spec Delta

## ADDED Requirements

### Requirement: Resume is decided by the connected adapter

When a Panel-mode agent has a prior session to continue, the system SHALL ask
its adapter for `session/load` only if the adapter's `initialize` response
advertises `loadSession`. The adapter registry's resume flag says which agent
types Knot tries to resume and reports the `resume` capability tag for. It
SHALL NOT be what decides whether a connected adapter is asked, so an adapter
version that predates loading is never sent a request it cannot answer.

Whether the session *resumed* SHALL be taken from the outcome of the connection,
not from whether a prior session was named:

- When `session/load` succeeds, the session is resumed. The agent is already
  registered, and neither the registration prompt nor the startup prompt SHALL
  be sent.
- When the adapter does not advertise `loadSession`, or refuses the load, the
  system opens a fresh session with `session/new`. That session SHALL be sent
  the registration prompt, and the startup prompt behind it, as any fresh
  session is.

Either way, the session id the connection ends with SHALL be the one persisted
for the next launch.

The standing instructions and persona reach the agent through its carrier on
every launch, resumed or not (see "Standing instructions through the agent's
system channel"). For `codex` that is `CODEX_CONFIG`, which `codex-acp`
re-applies on the resumed thread.

`codex` SHALL be resume-capable in the registry. `codex-acp` 2.0.0 advertises
`loadSession`, resumes the Codex thread on `session/load`, and replays its
history. Before this requirement `codex` was marked unsupported, and no Codex
conversation survived a restart.

#### Scenario: A Codex agent resumes its conversation

- **WHEN** a `codex` agent with a saved session is relaunched and its adapter
  advertises `loadSession`
- **THEN** the session is loaded, the prior conversation is shown, no
  registration or startup prompt is sent, and the agent's knot instructions and
  persona are still in effect through `CODEX_CONFIG`

#### Scenario: An adapter that cannot load starts fresh and registers

- **WHEN** an agent with a saved session is relaunched and its adapter does not
  advertise `loadSession`
- **THEN** no `session/load` is sent, a fresh session opens, and it is sent the
  registration prompt

#### Scenario: A refused load starts fresh and registers

- **WHEN** an agent with a saved session is relaunched, its adapter advertises
  `loadSession`, and the load is refused
- **THEN** a fresh session opens, it is sent the registration prompt, and its
  id is persisted in place of the refused one
