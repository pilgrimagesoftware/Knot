# Proposal

## Why

With "Restore last conversation" enabled, restarting an agent still throws its
conversation away and relaunches it with its initialization prompt. Users
restart an agent to recover it or to pick up a changed setting, not to lose
its context, and the setting reads as "keep my conversations". There is also
no deliberate way to start an agent over once a restart does keep the
conversation.

## What Changes

- Restart Agent and Restart All keep each agent's conversation when
  `restore-conversation-on-launch` is enabled, and start a new one when it is
  disabled. A panel agent reloads its ACP session. This is a deliberate departure from the Swift reference, which
  always starts fresh.
- New agent menu item **Restart with New Conversation** (⇧⌘R). It is in the row
  context menu and in the menu bar's Agents menu, and always starts a new
  conversation. It is hidden for shell agents, which have no conversation.
- The restart confirmations say whether the conversation will be kept.
- A loaded conversation shows its history: the prompts that `session/load`
  replays (`user_message_chunk`) were dropped, which left the replies run
  together with no prompts between them.
- Restarts caused by editing a launch-affecting field (folder, agent type,
  persona) still always start fresh.

- Conversations are restored at app launch for panel agents too:
  - the saved ACP session id is resolved first;
  - the panel connection loads it;
  - it is saved as soon as a session opens.

  Before, the saved id was never read back, so every panel agent started
  over after a relaunch.

Non-goals:
- Changing Resume or Fork.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-lifecycle`: Restart either keeps or discards the conversation,
  decided by the setting and by which item the user chose. Launch restore
  resolves the ACP session id first, the panel connection loads it, and it is
  persisted when a session opens.
- `agent-list-ui`: the row context menu gains Restart with New Conversation,
  with its visibility rule. Restart Agent and Restart All follow the setting.
- `app-menu`: the Agents menu's shortcut table gains ⇧⌘R.
- `acp-client`: user message chunks are delivered as their own update kind.
- `acp-panel-ui`: a loaded conversation shows its replayed prompts and replies.

## Impact

- `knot-agents`: a `restart_keeping_conversation` store operation beside
  `restart`, which is unchanged and still starts fresh.
- `knot-acp`: `SessionUpdate::UserMessageChunk`.
- `knot`:
  - a new `AgentMenuEntry` and action, with its binding in the Agents menu's
    fixed keys (`keymap::fixed` validates against them);
  - the two restart handlers choose the operation from the setting;
  - the confirmation strings move to l10n.
