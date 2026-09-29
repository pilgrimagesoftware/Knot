# Spec Delta

## ADDED Requirements

### Requirement: A resumed agent is registered

An agent whose Panel session resumes SHALL end up registered with the knot,
including when its earlier registration turn failed. Registration does not
survive a restart, and a resumed session is sent no first-launch registration
turn. Where the agent's adapter keeps its MCP URL's `?agent=` query (checked:
`claude`, `codex`), the connection SHALL register it, and the session SHALL be
sent no turn. For any other type, while the MCP server is enabled, the resumed
session SHALL be sent one turn: a single-line registration request, without the
instructions (already in the system channel) and without the startup prompt.
A session that fell back to a fresh one is registered as a fresh session is.

#### Scenario: A Claude agent resumes with no new turn

- **WHEN** a Claude agent's prior session loads after a restart
- **THEN** it is sent no prompt, and its connection registers it

#### Scenario: An agent on another adapter is asked once

- **WHEN** a Gemini agent's prior session loads after a restart with MCP enabled
- **THEN** it is sent exactly one prompt, the one-line registration request

#### Scenario: A failed first registration is recovered

- **WHEN** an agent whose first registration turn failed ("Not logged in") is
  resumed after a restart
- **THEN** it is registered, by its connection or by the one-line request
