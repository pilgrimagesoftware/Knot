## MODIFIED Requirements

### Requirement: Agent creation

The system SHALL create an agent from a working-directory path. Name defaults to
the last path component of the folder. Agent type defaults to `claude`.
Activation mode defaults to `passive`. Each agent receives a new unique id at
creation. A caller MAY override name, avatar, agent type, shell command,
persona, and activation mode, and MAY request insertion immediately after a
named sibling agent; absent a sibling, the new agent is appended.

#### Scenario: Create from folder with defaults

- **WHEN** an agent is created for `/Users/x/proj` with no other fields
- **THEN** its name is `proj`, its type is `claude`, its activation mode is
  `passive`, it has a fresh id
- **AND** it is appended to the agent list and added to a workspace

#### Scenario: Insert after a sibling

- **WHEN** an agent is created with `insert_after = <sibling id>`
- **THEN** it is placed directly after that sibling in both the master agent
  list and the sibling's workspace ordering

#### Scenario: Create an agent that starts with its workspace

- **WHEN** an agent is created with activation mode `active`
- **THEN** its activation mode is `active`, and it starts whenever its
  workspace opens

### Requirement: Durable versus runtime fields

The system SHALL persist only these agent fields: id, name, avatar, folder,
agent type, created-by, is-companion, shell command, persona id, activation
mode, and, conditionally, session id (see below). All other fields are
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

## ADDED Requirements

### Requirement: Activation mode

Every agent SHALL carry an activation mode of `active` or `passive`, which
decides when its session starts on its own:

- An `active` agent SHALL start when its workspace opens.
- A `passive` agent SHALL NOT start when its workspace opens. It starts the
  first time the user selects it and its pane takes focus, and then stays
  running until it is deactivated, removed, or its workspace closes.

Activation mode SHALL NOT change what an agent is or how it runs once
started: an activated `passive` agent is indistinguishable from an `active`
one. Changing an agent's activation mode SHALL NOT restart it, and SHALL NOT
start or stop it.

#### Scenario: Opening a workspace starts only its active agents

- **WHEN** a workspace holding two `active` and three `passive` agents opens
- **THEN** the two `active` agents start
- **AND** the three `passive` agents do not, and no adapter subprocess is
  spawned for them

#### Scenario: Selecting a passive agent starts it

- **WHEN** the user selects a `passive` agent that has not been activated
- **THEN** that agent starts, and behaves from then on exactly as an `active`
  agent would

#### Scenario: A passive agent stays running once activated

- **WHEN** an activated `passive` agent is deselected and another agent is
  selected
- **THEN** the first agent goes on running

#### Scenario: Changing the mode of a running agent leaves it running

- **WHEN** a running agent's activation mode is changed from `passive` to
  `active`, or the other way round
- **THEN** the agent keeps running, its session intact, and its restart token
  is unchanged

### Requirement: Deactivating an agent

The system SHALL let a running agent be deactivated: its terminal or ACP
session is torn down and its pane shows that it is stopped, while the agent
itself remains in its workspace with its name, folder, ordering, persona and
activation mode intact.

Deactivating SHALL NOT remove the agent, SHALL NOT remove its companions, and
SHALL NOT take it out of any workspace. A deactivated agent SHALL NOT start
again on its own for as long as its workspace stays open; selecting it starts
it, whatever its activation mode.

Deactivating an agent SHALL first deactivate every companion it owns, since a
companion has no session of its own to keep once its owner's is gone.

#### Scenario: Deactivate a running agent

- **WHEN** the user deactivates a running agent
- **THEN** its session is torn down, it remains in its workspace in the same
  position, and its pane reports that it is stopped

#### Scenario: A deactivated agent does not restart by itself

- **WHEN** an `active` agent is deactivated and the user selects other agents
  and returns to looking at the sidebar
- **THEN** that agent stays stopped until it is selected again

#### Scenario: Selecting a deactivated agent starts it

- **WHEN** the user selects a deactivated agent
- **THEN** it starts, in either activation mode

#### Scenario: Deactivating an owner takes its companions with it

- **WHEN** an agent owning two shell companions is deactivated
- **THEN** both companions are deactivated first, then the owner
