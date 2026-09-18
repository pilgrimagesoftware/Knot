## 1. Reach the store

- [ ] 1.1 Thread `Arc<Mutex<MessageStore>>` from `app_bootstrap` to
      `WorkspaceWindow`, through every path that opens one (workspace
      manager, command centre, direct open). Verify `make rust` builds and no
      opener was missed.

## 2. Deliver the nudge

- [ ] 2.1 Add per-agent last-nudged bookkeeping to the workspace window, and a
      pure decision function over (agent type, mcp enabled, latest unread,
      last nudged, idle, session ready) that says whether to nudge. Verify
      with tests covering: idle with a new message nudges; the same message
      twice does not; a busy agent does not; a shell agent does not; MCP off
      does not; no live session does not.
- [ ] 2.2 Call it from the repaint poll and send `CHECK_INBOX_PROMPT` through
      the panel session, reusing the composer's gate (ready slot, no pending
      permission, no turn in flight). Verify in the app: message one agent
      from another and watch the prompt arrive in the recipient's panel.
- [ ] 2.3 Verify the deferred case in the app: message an agent that is
      mid-turn, and confirm the nudge arrives after that turn ends rather
      than during it.

## 3. Verification

- [ ] 3.1 `make rust` passes clean.
- [ ] 3.2 Confirm a shell agent is never nudged, per `mcp-messaging`'s rule
      that shell agents cannot receive messages.
