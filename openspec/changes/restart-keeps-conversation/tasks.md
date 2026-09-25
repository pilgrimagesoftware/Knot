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

## 3. Gate

- [ ] 3.1 Run `make`. It must pass.
- [ ] 3.2 Run the app with "Restore last conversation" on. Restart a panel
  agent and confirm the conversation is still there, then use Restart with New
  Conversation and confirm it starts over.
