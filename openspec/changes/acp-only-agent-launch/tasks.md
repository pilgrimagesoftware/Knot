## 1. Fix MCP wiring over the ACP protocol

- [x] 1.1 Add an `mcp_url: Option<&str>` (or equivalent) parameter to
      `AcpClient::session_new`/`session_load` in `crates/knot-acp/src/client/mod.rs`,
      building `mcpServers: [{"name": "knot", "type": "http", "url": <url>}]`
      when present and `mcpServers: []` when not, and verify with a unit test
      asserting the request params for both cases.
- [x] 1.2 Thread the knot MCP URL from `Settings` (`mcp_server_enabled`,
      `mcp_server_port`) through `PanelSessionHandle::start`
      (`crates/knot/src/panel_session.rs`) into `AcpSession::start`
      (`crates/knot-terminal/src/acp_session/mod.rs`) and then into
      `session_new`/`session_load`, and verify by inspection that no call
      site still hard-codes an empty list.
- [x] 1.3 Add/extend an `AcpSession` or `AcpClient` test that starts a fake
      adapter and asserts the `session/new` request it receives carries the
      knot MCP server entry when enabled, and none when disabled.

## 2. Remove the dead CLI-arg MCP plumbing

- [x] 2.1 Delete `AdapterLaunch.mcp_config` and `mcp_config_args` in
      `crates/knot-agent-launch/src/builders.rs`, and the
      `command.arg(&launch.mcp_config)` line in
      `crates/knot-terminal/src/acp_session/mod.rs`, and verify
      `cargo build -p knot-agent-launch -p knot-terminal` succeeds with no
      leftover references.
- [x] 2.2 Update `plan_launch` so `LaunchPlan::Adapter` carries just the
      `AdapterConfig` (no `mcp_config` field), and update all call sites
      (`crates/knot/src/workspace_window/mod.rs`) accordingly; verify
      `cargo build -p knot` succeeds.

## 3. Narrow the terminal command builder to shell agents only

- [x] 3.1 Rewrite `build_agent_command` in
      `crates/knot-agent-launch/src/builders.rs` to only handle the shell
      case (custom command or empty), per the "Base command and user
      options" delta; verify with the existing
      `shell_agent_custom_command`/`shell_agent_no_custom_command_is_empty`
      tests (updated if signatures change) still passing.
- [x] 3.2 Delete `mcp_and_registration_args`, the non-shell arms of
      `mcp_arguments`/`inline_registration_arguments`/`persona_prompt`
      usage in the terminal path, and their now-orphaned tests in
      `crates/knot-agent-launch/src/registration.rs`/`escape.rs`, keeping
      only what `acp_registration_prompt` and shell-agent building still
      use; verify `cargo test -p knot-agent-launch` passes with the reduced
      test set matching the spec deltas (resume/registration/persona
      requirements removed).
- [x] 3.3 Rewrite `build_initialization_command` to the shell-only wrapper
      (no `KNOT_AGENT_ID`) per the new "Shell agent initialization wrapper"
      requirement, and update/replace its tests accordingly.
- [x] 3.4 Simplify `plan_launch` so it only returns `LaunchPlan::Terminal`
      for shell agents and `LaunchPlan::Adapter` for every other type with a
      registered adapter (an unadaptered non-shell type has no launch path -
      per the proposal's Impact note), and verify with updated tests in
      `crates/knot-agent-launch/src/builders.rs`.

## 4. Enforce Panel-only for non-shell agents

- [x] 4.1 In `AgentStore::create`/`edit` (`crates/knot-agents/src/store/`),
      coerce `view_mode` to `Panel` whenever `agent_type != "shell"`
      (creation, edit, and layout-restore/legacy-record loading), and add a
      test asserting a non-shell agent can never end up with `Terminal`.
- [x] 4.2 Remove the Terminal/Panel view-mode toggle button and its
      `toggle_view_mode` handler for non-shell agents in
      `crates/knot/src/workspace_window/mod.rs` (keep the underlying
      `ViewMode` type; only remove the user-facing toggle and any branch
      that would render a Terminal pane for a non-shell agent).
- [x] 4.3 Remove now-dead Terminal-mode rendering/session-management code
      paths for non-shell agents (`ensure_session`, `dispatch_key`,
      `resize_session_to_pane`, and related PTY/Grid wiring in
      `crates/knot/src/workspace_window/mod.rs` and
      `crates/knot/src/terminal_view.rs`) that only shell agents still
      exercise, keeping the shell-agent path intact; verify by reading
      through remaining callers that nothing references a non-shell PTY
      session.
- [x] 4.4 Update `crates/knot/src/dashboard.rs` and
      `crates/knot/src/settings_window/` for any Terminal-vs-Panel
      display/setting surfaced for non-shell agents, removing it.

## 5. Verification

- [x] 5.1 Run `make rust` (fmt + clippy + test + build, whole workspace) and
      confirm it passes clean.
- [ ] 5.2 (blocked on a human at the keyboard - see note below) Manually
      launch one ACP-capable agent type (e.g. `opencode`) via
      the `run` skill, confirm its registration prompt succeeds and it can
      call `set-status` without a "tool not available" response, using the
      logging added in `crates/knot-acp/src/transport/mod.rs` (spawn
      command, request/response) to verify the MCP server entry actually
      reaches the adapter.
- [ ] 5.3 (blocked on a human at the keyboard - see note below) Confirm a
      shell companion agent still launches into a working PTY terminal,
      unaffected by this change.

## 6. Defects found during verification

Surfaced by task 5.2's manual session once every non-shell agent started
launching over ACP at window-open. Each is a pre-existing bug in code this
change made load-bearing, not a regression it introduced.

- [x] 6.1 Repaint the panel pane when its session slot changes lifecycle
      phase, not only when a `Ready` handle reports new events
      (`WorkspaceWindow::panel_needs_repaint`). Without this a
      `Connecting -> Failed` transition never called `cx.notify()`, so a
      failed connection (gemini, on a host with no configured API key) sat
      on "Connecting to agent…" forever and made the 20s `CONNECT_TIMEOUT`
      look like it had never fired.
- [x] 6.2 Let the conversation actually scroll and wrap: the message list
      was `size_full().overflow_y_hidden()` *inside* the caller's scroll
      container, pinning it to one viewport-height and clipping the rest,
      and nothing in the chain carried `min_w_0()`, so wide content
      stretched the pane until the Send button left the window.
- [x] 6.3 Parse a tool call's `content` array off `tool_call`/
      `tool_call_update` (`knot_acp::ToolCallContent`) and key the card's
      in-progress placeholder on `status` rather than "no output yet".
      ACP has no `tool_call_result` or `diff` session-update kind, so
      `ToolCallCard::result` was never set and every card read "Running…"
      even after its status said `completed`. Verified against
      <https://agentclientprotocol.com/protocol/tool-calls>.
- [x] 6.4 Show the agent type on each sidebar row, so a one-character
      avatar isn't the only indication of which agent is running there.
- [x] 6.5 Persist agent removal and tear down the ACP session on it.
      `AgentStore::remove` only mutates memory and nothing writes settings
      on quit, so a removed agent returned on the next launch; the removal
      also left the adapter subprocess running, since `remove_session` only
      knew about PTY sessions.
- [ ] 6.6 (blocked on a human at the keyboard - see note below) Confirm
      6.1-6.5 live: gemini now reports its connection error
      instead of hanging; a long response scrolls and wraps with the Send
      button in place; a finished tool call stops reading "Running…"; the
      sidebar names each agent's type; "Remove Agent" removes it and it
      stays removed across a relaunch.

## Note on the remaining manual tasks

5.2, 5.3 and 6.6 all need someone looking at the running app. An agent
session cannot stand in for that here: Claude Code's terminal has no
Screen Recording permission, so it cannot see the window, and GPUI
publishes no labelled accessibility tree (every control comes through as
an unnamed `button`), so the UI cannot be driven blind either. What was
verified without pixels is recorded in `HANDOFF.md`.
