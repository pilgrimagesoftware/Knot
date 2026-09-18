# Hand-off: acp-only-agent-launch

Branch: `acp-only-agent-launch`, pushed to origin, no PR opened yet.
Every open bug from the previous hand-off is fixed in code; what remains is
live confirmation in the running app. Delete this file once the checklist
below is signed off and before merging.

## Done and verified (build/clippy/test green, user-tested Claude + OpenCode)

- OpenSpec change `openspec/changes/acp-only-agent-launch/` implemented
  (task groups 1-4, plus group 6's follow-up defects).
- Real root cause of `set-status`/MCP-never-worked-for-ACP-agents: fixed.
  `AcpClient::session_new`/`session_load` now send the correct ACP
  `mcpServers` shape (verified against agentclientprotocol.com/protocol/v1/schema,
  not guessed) and only include Knot's HTTP MCP server when the agent's
  `initialize` response declares `mcpCapabilities.http` support.
- Non-shell agents (claude/codex/opencode/gemini/copilot) launch exclusively
  via ACP; Terminal/Panel toggle removed; shell/companion agents unaffected.
- Logging: `crates/knot-acp/src/transport/mod.rs` prefixes every
  request/response line with `[program]` so concurrent connections (every
  agent now connects at window-open) can be told apart in stderr; `knot-mcp`
  logs every RPC method + tool name for `tools/call` + the full tool list for
  `tools/list`.
- Confirmed working live: Claude and OpenCode connect and get MCP tools.

## Fixed, pending live confirmation

Each was diagnosed to a concrete mechanism and has unit coverage; none has
been watched in the running app. See `tasks.md` group 6 for the same list.

1. **Gemini hung on "Connecting to agent…" past the 20s timeout.** The
   timeout was firing all along - the repaint poll in
   `workspace_window/mod.rs` only called `cx.notify()` for
   `PanelSessionSlot::Ready(handle) if handle.take_dirty()`, so a slot
   moving `Connecting -> Failed` was never drawn. Claude and OpenCode only
   looked fine because their registration prompt sets the dirty flag on
   success. Now `panel_needs_repaint` also repaints on a phase change.
   Note: probing `gemini --acp --skip-trust` 0.46.0 by hand here, the
   handshake is fine and `session/new` answers
   `-32000 "Gemini API key is missing or not configured."` - so the
   expected post-fix behaviour on an unauthenticated host is a visible
   "Failed to connect: …", not a successful session. `--skip-trust` is
   still honoured; that part of the old note holds.

2. **Panel content clipped, no scroll, Send button off-screen.** The
   message list was `size_full().overflow_y_hidden()` *inside* the
   caller's `overflow_y_scroll` container, which pins it to one
   viewport-height and clips everything past it. Horizontally, nothing in
   the chain carried `min_w_0()`, so a markdown table widened the pane
   until the input row's Send button left the window. Fixed in
   `panel_view/mod.rs` (`w_full`/`min_w_0`, no inner overflow hide;
   unwrappable markdown scrolls sideways in its own container) and in
   `workspace_window/mod.rs`'s content column.

3. **Tool-call cards stuck on "Running…" after "completed".** Wire-format
   mismatch, not a label mismatch: ACP has no `tool_call_result` or `diff`
   `sessionUpdate` kind. A call's output arrives as a `content` array on
   `tool_call`/`tool_call_update`
   (<https://agentclientprotocol.com/protocol/tool-calls>), so
   `ToolCallCard::result` was never set and the body fell through to the
   "no output yet" placeholder forever. `knot_acp::ToolCallContent` now
   parses that array (text, diff, terminal blocks), the card keys its
   placeholder on `status`, and partial updates no longer blank out fields
   they omit.

4. **Sidebar rows didn't name the agent type.** Each non-shell row now
   shows its type label (`SettingsWindow::agent_type_label`) beside the
   name. The row tuple became an `AgentRow` struct on the way.

5. **"Remove Agent" appeared to do nothing.** Two real defects on that
   path: `AgentStore::remove` only mutates memory and nothing persists
   settings on quit, so the agent came back on the next launch; and
   `remove_session` only knew about PTY sessions, so a Panel agent's
   adapter subprocess kept running. Both fixed via
   `WorkspaceWindow::remove_agent`, with round-trip tests in
   `crates/knot/src/tests/mod.rs`. If the confirm dialog still never
   *appears* on click, that is a separate GPUI modal-over-context-menu
   problem and needs the live repro the previous hand-off called for -
   nothing in the static read of `PopupMenu::confirm`/`dismiss` or
   `AlertDialog` explains it, and "Restart Agent" uses the identical shape.

## Live checklist before opening the PR

- [ ] Gemini shows "Failed to connect: …" (or connects) rather than hanging.
- [ ] A long response scrolls, wraps, and leaves the Send button in place.
- [ ] A finished tool call stops reading "Running…" and shows its output.
- [ ] Sidebar rows name each agent's type.
- [ ] "Remove Agent" removes the row, and it stays gone after a relaunch.
- [ ] A shell companion still launches into a working PTY terminal
      (tasks.md 5.3).

## Environment notes

- `cargo +nightly fmt` on this machine resolved to the *stable* rustfmt
  binary. `rustup toolchain install nightly --profile minimal --component
  rustfmt` fixes it; either way the nightly binary works directly:
  `~/.rustup/toolchains/nightly-aarch64-apple-darwin/bin/rustfmt --edition 2024 <files>`
- `knot-discovery`'s `rapid_child_creation_coalesces_to_one_rescan` test is
  known-flaky in a sandboxed shell (passes outside it) - pre-existing, not
  caused by this branch.
