# Hand-off: acp-only-agent-launch

Branch: `acp-only-agent-launch`, pushed to origin, no PR opened yet
(deliberately — still has open bugs below). Delete this file once
everything below is resolved and before merging.

## Done and verified (build/clippy/test green, user-tested Claude + OpenCode)

- OpenSpec change `openspec/changes/acp-only-agent-launch/` fully implemented
  (all 5 task groups checked off in `tasks.md`).
- Real root cause of `set-status`/MCP-never-worked-for-ACP-agents: fixed.
  `AcpClient::session_new`/`session_load` now send the correct ACP
  `mcpServers` shape (verified against agentclientprotocol.com/protocol/v1/schema,
  not guessed) and only include Knot's HTTP MCP server when the agent's
  `initialize` response declares `mcpCapabilities.http` support.
- Non-shell agents (claude/codex/opencode/gemini/copilot) launch exclusively
  via ACP; Terminal/Panel toggle removed; shell/companion agents unaffected.
- Logging added: `crates/knot-acp/src/transport/mod.rs` prefixes every
  request/response line with `[program]` so concurrent connections (every
  agent now connects at window-open) can be told apart in stderr; `knot-mcp`
  logs every RPC method + tool name for `tools/call` + the full tool list for
  `tools/list`.
- Confirmed working live: Claude and OpenCode connect and get MCP tools.

## Open bugs (in priority order)

1. **Gemini hangs on "Connecting to agent…" past the 20s timeout.**
   Not yet diagnosed — the timeout itself not firing is suspicious (should
   show "Failed to connect: Timeout" after `CONNECT_TIMEOUT` in
   `crates/knot-terminal/src/acp_session/mod.rs`). Next step: rerun with the
   new `[gemini]`-prefixed logging (commit `c82b43f`) and grep stderr for
   `gemini` to see exactly where it stalls - does `initialize` ever get a
   response? Does gemini's own stderr print anything (prefixed `[gemini]`)?
   If `initialize` truly never responds, check whether `--skip-trust` is
   actually being honored by the installed gemini CLI version (the code
   comment in `crates/knot-agent-launch/src/adapter.rs` claims this was
   "confirmed live" at some point - may have changed).

2. **Panel content: text clipped instead of wrapped, no scroll, send button
   pushed off-screen.** Not investigated. Likely in
   `crates/knot/src/panel_view/mod.rs` (`render_message`/`render_tool_call_card`
   or similar) - missing word-wrap on some content type (looked like a
   markdown table in the screenshot) and the message list container missing
   `overflow_y_scroll()`/`min_w_0()` (see `knot-ui-conventions.md`'s "Flex
   overflow" rule - a wrapping child needs `min_w_0()` on its immediate flex
   parent, not just `flex_1()`).

3. **Tool-call cards stuck showing "Running…" after being marked
   "completed".** Not investigated. Likely a status-label mismatch in
   `panel_view/mod.rs`'s tool-call card rendering vs. `panel_state`'s status
   field, or a `SessionUpdate::ToolCallUpdate` status string not matching
   what the render code checks for.

4. **Sidebar agent cells don't show which agent type they're running**
   (e.g. "O" doesn't say OpenCode). Not investigated - likely just needs the
   agent-type name added to the row in `workspace_window/mod.rs`'s
   `agent_rows` rendering.

5. **"Remove Agent" context-menu item does nothing.** Investigated earlier
   this session with no root cause found by reading `AgentStore::remove`
   (correct, tested) and the context-menu wiring in
   `crates/knot/src/workspace_window/mod.rs` (structurally matches the
   working "Restart Agent" item right above it). Needs live reproduction:
   does the confirm dialog even appear on click? If not, likely a GPUI
   modal-over-context-menu interaction issue (opening `window.open_alert_dialog`
   from inside a context-menu's `on_click` callback).

## Environment notes

- `cargo +nightly fmt` on this machine resolves to the *stable* rustfmt
  binary (environment quirk, not a code issue). Format with the nightly
  binary directly:
  `~/.rustup/toolchains/nightly-aarch64-apple-darwin/bin/rustfmt --edition 2024 <files>`
- Run the app from this worktree, not the main checkout:
  `cd /Users/paulyhedral/Projects/Code/Knot/Worktrees/acp-only-agent-launch && cargo run -p knot`
- `knot-discovery`'s `rapid_child_creation_coalesces_to_one_rescan` test is
  known-flaky in the sandboxed Bash tool (passes outside sandbox) -
  unrelated pre-existing issue, not caused by this branch.
