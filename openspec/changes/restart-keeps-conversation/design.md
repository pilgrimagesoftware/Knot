# Design

## Context

- `AgentStore::restart` clears session id, ACP session id, resume-session id
  and the fork flag, then regenerates the restart token.
- The window then tears the session down (`remove_session`).
- The next frame's `ensure_panel_session` reconnects, passing the agent's
  `acp_session_id` as `prior_session_id`:
  - when it is present, the connection loads that session with the short
    re-registration prompt;
  - when it is absent, it starts a new session with the full initialization
    prompt.
- `connect_into` records the new session's id with `set_acp_session_id` as
  soon as a session is live. A restart during this launch therefore always
  has an id to keep.

## Decisions

### A second store operation, not a flag on `restart`

`restart_keeping_conversation(id)` keeps `session_id` and `acp_session_id`,
sets `resume_session_id` to `session_id`, clears the fork flag, and recreates
the terminal. `restart(id)` keeps its meaning (start fresh), so its callers
don't change:
- the launch-affecting edit path in `store/edit.rs`;
- Restart with New Conversation.

A `restart(id, Conversation)` parameter was considered. It would touch every
existing caller and test only to have most of them pass the same value, and
the two operations read better as names at the call site.

The fork flag is cleared in both: a fork is a one-time instruction for the
first launch, and by the time an agent can be restarted it has already
happened.

### The setting is read at the call site

Restart Agent and Restart All read `restore_conversation_on_launch` when the
user confirms, not when the menu opens. This follows the rule the settings
code already keeps: read the shared surface at the moment of use. A
`WorkspaceWindow::restart_agents(ids, keep_conversation, cx)` helper does the
store operation, the session teardown and the single persist for both, so the
row and bulk paths cannot drift apart.

### Menu entry and key

- `AgentMenuEntry::RestartWithNewConversation` goes directly after
  `RestartAgent`, in the same group. It is filtered `owner_only && !is_shell`.
- It is mapped to a new `AgentMenuRestartWithNewConversation` action, which
  keeps the menu bar and row menu produced from one item set.
- ⇧⌘R is added to `agent_menu_key_bindings`. That makes it a fixed shortcut:
  the Keyboard settings tab refuses to rebind anything onto it.
- No gpui-kit input context binds ⇧⌘R.

### Confirmations

The row's Restart Agent confirmation is currently a hard-coded English
`format!`, even though `menu.agent.confirm.restart_body` exists. Both restart
items now build their text through l10n. There are three bodies:
- keeping the conversation;
- clearing it (the existing text);
- starting over.

Restart All gets a keep-conversation body beside its existing one.

## Risks / Trade-offs

- [Risk] The adapter no longer has the session, or its type does not support
  resume (`codex`'s `supports_resume` is false). → `connect_into` already falls
  back to a new session without surfacing an error (`agent-lifecycle`: "Resume
  fails for the ACP adapter"). A test pins that the store keeps the id, and the
  existing connect fallback covers the rest.
- [Trade-off] With the setting off, Restart with New Conversation does the same
  as Restart Agent. It stays visible anyway, so the menu keeps one shape.
