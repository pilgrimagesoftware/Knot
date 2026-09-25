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

1. If the agent's persisted ACP session id is present, use it directly.
2. Otherwise, if its persisted session id is present, use that.
3. Otherwise, look up the most recent session for that agent's `(folder,
   agent type)` via the conversation-history provider registry, and use its
   id if found.

A Panel-mode agent's connection SHALL load its live ACP session id when it
has one, and otherwise the resolved resume-session id, so a conversation
restored at launch is loaded rather than replaced by a new session. The ACP
session id comes first in the resolution because it is the only one a
Panel-mode agent records. Without it, every such agent fell through to the
history lookup, which picks another agent's conversation when two share a
folder.

When `restore-conversation-on-launch` is enabled, an agent's ACP session id
SHALL be persisted as soon as its session opens, not left for the next
roster change, so quitting straight afterwards still restores it.

If neither step yields an id — `restore-conversation-on-launch` is disabled,
no persisted session id exists, no history provider exists for the agent's
type, or no session is found — resume-session id SHALL remain at its default
(none) and the agent launches fresh, with no error surfaced. In all cases the
agent's runtime session id itself remains unset by this resolution; it is
set by the normal resume flow when the terminal actually resumes.

This resolution applies only to loading agents for layout restore (e.g. cold
app launch). A manual Restart of an already-running agent performs no history
lookup; whether it keeps the agent's conversation is decided by the Restart
requirement. A manual restart does not alter the agent's persisted session
id.

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

#### Scenario: A panel agent's conversation survives a relaunch

- **WHEN** `restore-conversation-on-launch` is enabled, a Panel-mode agent
  with ACP session id `a1` is persisted, and the app is relaunched
- **THEN** the agent's resume-session id is `a1`, and its connection loads
  session `a1` instead of creating a new one

#### Scenario: Manual restart is unaffected

- **WHEN** `restore-conversation-on-launch` is enabled and a running agent is
  manually restarted from the UI
- **THEN** no history lookup is performed, the agent keeps the conversation it
  had per the Restart requirement, and its persisted session id is untouched
  until the next persist

#### Scenario: Legacy record without registry fields

- **WHEN** a persisted agent record predates the description, capabilities
  and cost-tier fields
- **THEN** it loads with an empty description, no capability tags, and cost
  tier `medium`, and nothing about how it launches changes

### Requirement: Restart
Restarting an agent SHALL preserve its id and regenerate its restart token,
which forces its terminal session to be destroyed and recreated, and — when
the agent is in Panel mode — forces its ACP session to be closed and a new
connection made. Restart SHALL clear the fork flag, reset state to Idle, set
registered to false, and clear the terminal title.

A restart either keeps the agent's conversation or starts a new one:

- **Keeping the conversation** SHALL keep session id and ACP session id and
  set resume-session id to the session id, so the relaunched agent resumes the
  conversation it had: a Panel-mode agent loads its ACP session again. An
  agent that has no session yet, or whose adapter cannot resume, starts fresh
  with no error surfaced. A shell agent has no conversation, so for it the two
  are the same.
- **Starting a new conversation** SHALL clear session id, ACP session id and
  resume-session id, so the agent launches fresh, with its initialization
  prompt.

A user-initiated Restart (the agent's Restart Agent item and the sidebar's
Restart All) SHALL keep the conversation when
`restore-conversation-on-launch` is enabled, and start a new one when it is
disabled. Restart with New Conversation SHALL always start a new one. A
restart triggered by editing a launch-affecting field SHALL always start a
new one, because the old session belongs to a folder, agent type or persona
the agent no longer has.

The Swift reference always starts a new conversation on restart. Knot keeps
it when the user has asked for conversations to be restored, so that a
restart - done to recover a stuck agent, or to pick up a changed setting -
does not also throw away its context.

#### Scenario: Restart keeps identity, drops session

- **WHEN** `restore-conversation-on-launch` is disabled and a registered agent
  with session id `s1` is restarted
- **THEN** its id is unchanged, its restart token differs, its session id is
  cleared, it is unregistered, and its state is Idle

#### Scenario: Restart keeps the conversation when restore is on

- **WHEN** `restore-conversation-on-launch` is enabled and a Panel-mode agent
  with ACP session id `a1` is restarted
- **THEN** its ACP session is closed, its ACP session id is still `a1`, and on
  relaunch it loads session `a1` rather than starting a new one
- **AND** it is unregistered and its state is Idle

#### Scenario: Restart with New Conversation always starts fresh

- **WHEN** `restore-conversation-on-launch` is enabled and the user chooses
  Restart with New Conversation for an agent with ACP session id `a1`
- **THEN** its session id, ACP session id and resume-session id are cleared,
  and it relaunches in a new session with its initialization prompt

#### Scenario: Restart a Panel-mode agent
- **WHEN** `restore-conversation-on-launch` is disabled and the user restarts
  an agent currently in Panel mode
- **THEN** its active ACP session is closed, its ACP session id is cleared,
  and a new ACP session is created on relaunch

#### Scenario: A launch-affecting edit starts fresh

- **WHEN** `restore-conversation-on-launch` is enabled and the user changes a
  running agent's folder
- **THEN** it restarts with its session id, ACP session id and resume-session
  id cleared
