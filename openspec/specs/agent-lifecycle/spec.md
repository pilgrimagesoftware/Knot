# agent-lifecycle Specification

## Purpose
Defines how agents and their shell companions are created, edited, restarted,
resumed, and removed; which agent fields are durable versus runtime-only; and
how an agent is placed within workspaces. This is the contract the Rust port
satisfies; behavior is the intended target, derived from the Swift reference
app with the dual terminal engine removed (Ghostty only).

## Requirements

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

### Requirement: Three distinct status fields

The system SHALL keep three independent per-agent strings: `state` (the
automatic state machine value), `status_text` (set only by the agent via the
`set-status` MCP tool), and `terminal_title` (from terminal escape sequences,
with leading spinner/indicator glyphs stripped). The terminal header SHALL show
`status_text` when non-empty, otherwise `terminal_title`.

#### Scenario: Header prefers agent-set status

- **WHEN** `status_text` is `"Refactoring auth"` and `terminal_title` is `"zsh"`
- **THEN** the header shows `"Refactoring auth"`

### Requirement: Shell companions

The system SHALL support shell companion agents bound to an owner agent via
created-by set to the owner id and is-companion true. A non-companion agent MAY
own companions; a companion MUST NOT own companions. Creating a shell companion
places it after its owner and enters a split layout pairing owner and companion.

#### Scenario: Companion is bound to its owner

- **WHEN** a shell companion is created for owner `A`
- **THEN** the companion's created-by is `A`, is-companion is true, agent type
  is `shell`, and it is inserted after `A`

### Requirement: Agent removal

When an agent is removed, the system SHALL first remove every companion it owns,
then, if the agent is registered with MCP, unregister it, then tear down its
terminal session and any per-agent notification tracking, then remove it from
all workspaces and from the master agent list. Removal SHALL re-select or
collapse split panes so no pane references the removed agent.

#### Scenario: Removing an owner cascades to companions

- **WHEN** owner `A` with companions `B` and `C` is removed
- **THEN** `B` and `C` are removed first, then `A`
- **AND** none remain in any workspace or the master list

#### Scenario: Removing a registered agent unregisters it

- **WHEN** a registered agent is removed
- **THEN** an MCP unregister is issued for its id before teardown

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

### Requirement: Resume session

Resuming an agent into a session SHALL set both resume-session id and session id
to the target session, clear the fork flag, and then perform a restart.

#### Scenario: Resume targets a prior session

- **WHEN** an agent is resumed into session `s2`
- **THEN** resume-session id and session id are `s2`, fork is false, and the
  terminal session is recreated

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

### Requirement: Ordering and workspace placement

The system SHALL let an agent be reordered within its workspace and moved to
another workspace. A new agent inherits the workspace of its created-by or
insert-after source when one exists; otherwise it joins the current workspace.
If a workspace has no active agent, a newly added agent becomes its active
agent.

#### Scenario: New agent inherits source workspace

- **WHEN** an agent is created with `insert_after = X` and `X` lives in
  workspace `W`
- **THEN** the new agent is added to `W`, not the current workspace

### Requirement: Bench deployment

Deploying a bench agent SHALL verify the target folder exists and is a
directory. If the check fails, the bench entry SHALL be removed and no agent
created. If it succeeds, an agent is created from the bench entry's folder,
name, avatar, agent type, shell command, and persona.

#### Scenario: Stale bench entry is pruned

- **WHEN** a bench agent whose folder no longer exists is deployed
- **THEN** no agent is created and the bench entry is removed

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
again on its own for as long as its workspace stays open, with two
exceptions: selecting it starts it, whatever its activation mode, and a
direct message addressed to it (see `mcp-messaging`'s "Direct send activates
a deactivated recipient") starts it the same way. Nothing else - a broadcast
reaching it, another agent's session starting, a poll or a timer - starts a
deactivated agent.

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

#### Scenario: A direct message starts a deactivated agent

- **WHEN** another agent in the same workspace sends a direct message to a
  deactivated agent
- **THEN** the deactivated agent starts, in either activation mode

#### Scenario: A broadcast does not start a deactivated agent

- **WHEN** another agent broadcasts to its workspace and a deactivated agent
  is among the eligible recipients
- **THEN** that agent stays stopped
