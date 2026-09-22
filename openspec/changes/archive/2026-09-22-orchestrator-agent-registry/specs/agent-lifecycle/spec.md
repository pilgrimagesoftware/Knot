# Spec Delta

## MODIFIED Requirements

### Requirement: Durable versus runtime fields

The system SHALL persist only these agent fields: id, name, avatar, folder,
agent type, created-by, is-companion, shell command, persona id, activation
mode, description, capabilities, cost tier, and, conditionally, session id
(see below).

Description, capabilities and cost tier are the registry fields defined by
`agent-registry`. A persisted record written before they existed SHALL load
with an empty description, no capability tags, and cost tier `medium`, the
same way a record predating activation mode loads as `active`. All other fields are
runtime-only and MUST reset to defaults when agents are loaded: state (Idle),
status text (empty), registered (false), pending-start (false), terminal title
(empty), resume-session id (none), hook metadata (empty), git stats (none),
activated (false).

A persisted agent record with no activation mode SHALL load as `active`.
Records written before activation mode existed describe agents that started
with their workspace, and MUST go on doing so.

When the `restore-conversation-on-launch` scalar setting is enabled,
persisting an agent SHALL additionally record its current session id. When
the setting is disabled, session id SHALL NOT be persisted (recorded as
none), regardless of the agent's runtime session id at persist time.

When `restore-conversation-on-launch` is enabled, loading agents as part of a
layout restore SHALL, for each restored agent, resolve a resume-session id
as follows, applied before that agent's terminal session is launched:

1. If the agent's persisted session id is present, use it directly.
2. Otherwise, look up the most recent session for that agent's `(folder,
   agent type)` via the conversation-history provider registry, and use its
   id if found.

If neither step yields an id — `restore-conversation-on-launch` is disabled,
no persisted session id exists, no history provider exists for the agent's
type, or no session is found — resume-session id SHALL remain at its default
(none) and the agent launches fresh, with no error surfaced. In all cases the
agent's runtime session id itself remains unset by this resolution; it is
set by the normal resume flow when the terminal actually resumes.

This resolution applies only to loading agents for layout restore (e.g. cold
app launch). It SHALL NOT apply to a manual "Restart" of an already-running
agent, which continues to always clear session id and resume-session id per
the Restart requirement; a manual restart does not alter the agent's
persisted session id.

#### Scenario: Reload drops runtime state

- **WHEN** an agent that was Working with a session id is persisted with
  `restore-conversation-on-launch` disabled and reloaded
- **THEN** the reloaded agent is Idle, unregistered, with no session id and no
  terminal title

#### Scenario: A passive agent that ran is passive again after a reload

- **WHEN** a `passive` agent that was activated and running is persisted and
  reloaded
- **THEN** its activation mode is still `passive` and it is not activated,
  so it does not start until it is selected again

#### Scenario: Legacy record without companion fields

- **WHEN** a persisted agent record predates the created-by / is-companion
  fields
- **THEN** it loads with created-by unset and is-companion false, and agent type
  defaults to `claude` if absent

#### Scenario: Legacy record without an activation mode

- **WHEN** a persisted agent record predates the activation-mode field
- **THEN** it loads as `active` and starts with its workspace, exactly as it
  did before the field existed

#### Scenario: Restore-conversation setting off leaves resume-session id unset

- **WHEN** `restore-conversation-on-launch` is disabled and an agent for
  folder `/Users/x/proj` with a matching prior `claude` session is loaded at
  layout restore
- **THEN** the reloaded agent's resume-session id is unset and it launches a
  fresh conversation

#### Scenario: Enabling the setting persists the agent's exact session id

- **WHEN** `restore-conversation-on-launch` is enabled and a `claude` agent
  with runtime session id `s7` is persisted
- **THEN** the saved agent record's session id is `s7`

#### Scenario: Disabling the setting stops persisting session id

- **WHEN** `restore-conversation-on-launch` is disabled and a `claude` agent
  with runtime session id `s7` is persisted
- **THEN** the saved agent record's session id is none, even though the
  agent's runtime session id was `s7` at persist time

#### Scenario: Restore prefers the agent's own persisted session id

- **WHEN** `restore-conversation-on-launch` is enabled and an agent for
  folder `/Users/x/proj` is loaded at layout restore with a persisted session
  id `s7`, and the conversation-history provider for that folder reports a
  different most-recent session `s9`
- **THEN** the reloaded agent's resume-session id is set to `s7` (its own
  exact session), not `s9`

#### Scenario: Restore falls back to history lookup with no persisted id

- **WHEN** `restore-conversation-on-launch` is enabled, an agent of type
  `claude` for folder `/Users/x/proj` has no persisted session id, and the
  conversation-history provider for `claude` reports a most-recent session
  `s9` for that folder
- **THEN** the reloaded agent's resume-session id is set to `s9` before its
  terminal launches, and session id remains unset until the resume completes

#### Scenario: No history available falls back to fresh launch

- **WHEN** `restore-conversation-on-launch` is enabled, an agent of type
  `shell` (no history provider) has no persisted session id, and is loaded at
  layout restore
- **THEN** the reloaded agent's resume-session id remains unset and it
  launches fresh, with no error

#### Scenario: ACP session id has no analogue in terminal mode

- **WHEN** an agent has never been in Panel mode
- **THEN** its ACP session id remains none and is never persisted

#### Scenario: Resume fails for the ACP adapter

- **WHEN** layout restore attempts `session/load` for a Panel-mode agent and
  the adapter reports the session no longer exists
- **THEN** the system starts a fresh ACP session for that agent instead of
  surfacing an error to the user

#### Scenario: Manual restart is unaffected

- **WHEN** `restore-conversation-on-launch` is enabled and a running agent is
  manually restarted from the UI
- **THEN** its runtime session id and resume-session id are cleared as usual,
  no history lookup is performed, and its persisted session id is untouched
  until the next persist

#### Scenario: Legacy record without registry fields

- **WHEN** a persisted agent record predates the description, capabilities
  and cost-tier fields
- **THEN** it loads with an empty description, no capability tags, and cost
  tier `medium`, and nothing about how it launches changes

### Requirement: Edit triggers restart only for launch-affecting changes

Editing an agent's name, avatar, description, capabilities, or cost tier
SHALL NOT restart it: none of them changes how the session runs, and
re-tagging an agent mid-job would otherwise throw away the work it is doing.
Changing its folder, agent type, or persona SHALL restart it. When the folder changes and companion
relocation is requested, each companion that shared the old folder SHALL be
moved to the new folder and restarted.

#### Scenario: Rename does not restart

- **WHEN** only the agent's name changes
- **THEN** the terminal session is not recreated

#### Scenario: Folder change restarts and can relocate companions

- **WHEN** the agent's folder changes with relocate-companions requested
- **THEN** the agent restarts, and each companion at the old folder moves to the
  new folder and restarts

#### Scenario: Re-tagging a working agent does not interrupt it

- **WHEN** a capability tag is added to an agent that is Working
- **THEN** its session is not recreated and it keeps working
