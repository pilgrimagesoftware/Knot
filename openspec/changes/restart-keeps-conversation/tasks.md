# Tasks

## 1. Store

- [x] 1.1 Add `AgentStore::restart_keeping_conversation`. Verify with
  `knot-agents` tests: it keeps the id, session id and ACP session id, sets the
  resume-session id to the session id, clears the fork flag, regenerates the
  restart token, and resets state and registration like `restart`.

## 2. Menu item and handlers

- [x] 2.1 Add `AgentMenuEntry::RestartWithNewConversation`, its label, its
  place after Restart Agent (owner-only, non-shell), the
  `AgentMenuRestartWithNewConversation` action and ⇧⌘R. Verify with the
  `agent_context_menu`, `agents_menu`, `menu_key_equivalents` and keymap
  tests, updated for the new entry.
- [x] 2.2 Add `WorkspaceWindow::restart_agents(ids, keep_conversation, cx)` and
  route Restart Agent, Restart All and Restart with New Conversation through
  it. The first two read `restore_conversation_on_launch` when their prompt
  opens.
  Verify with gpui tests on a real workspace window that, with the setting on,
  Restart keeps the ACP session id and Restart with New Conversation clears
  it, and that with the setting off Restart clears it.
- [x] 2.3 Move the restart confirmations to l10n, with keep, clear and
  start-over bodies and a keep body for Restart All. Verify with the l10n
  key-resolution tests.

## 3. Replayed history

- [x] 3.1 Parse `user_message_chunk` as `SessionUpdate::UserMessageChunk`, and
  fold it into user messages between turns without starting one. Verify with
  a `knot-acp` parser test and `panel_state` replay tests: turns stay apart,
  chunks join, no turn starts, and a mid-turn echo is ignored.

- [x] 3.2 Open a conversation list that fills from empty at its newest row.
  Verify with a `panel_scroll` test on the real row layout, which fails
  without the change.

## 4. Restore on relaunch

- [x] 4.1 Resolve the launch resume-session id from the persisted ACP session
  id first, and have the panel connection load `Agent::session_to_load` (the
  live ACP id, else the resolved one). Verify with a `startup` test and a
  `knot-agents` test of `session_to_load`.
- [x] 4.2 Persist the roster when a panel session's id is recorded. Verify by
  relaunching the app with the setting on (5.2); no fake-adapter window test
  exists to drive a real connect.

## 5. Gate

- [x] 5.1 Run `make`. It must pass.
- [ ] 5.2 Run the app with "Restore last conversation" on. Restart a panel
  agent and confirm the conversation, prompts included, is still there, then use Restart with New
  Conversation and confirm it starts over. Quit and relaunch, and confirm
  the conversation is restored.
