# Spec Delta

## ADDED Requirements

### Requirement: A loaded session keeps the id it was loaded by

A successful `session/load` SHALL identify the session by the `sessionId` its
response carries when it carries one, and otherwise by the `sessionId` the
request named. ACP's `LoadSessionResponse` defines no `sessionId`: a loaded
session continues under the id it was asked for. A response without one SHALL
NOT be treated as a failure.

`codex-acp` 2.0.0 answers `session/load` with its models, modes and config
options and no `sessionId`. Treating that as an error made every Codex load
fall back to a fresh session.

#### Scenario: A load response with no session id

- **WHEN** the caller loads session `thread-7` and the agent's successful
  response has no `sessionId`
- **THEN** the loaded session is identified as `thread-7`

#### Scenario: A load response naming its session

- **WHEN** the agent's successful load response carries a `sessionId`
- **THEN** the loaded session is identified by that id
