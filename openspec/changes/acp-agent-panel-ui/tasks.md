## 1. Spike: confirm ACP adapter invocation per agent

- [x] 1.1 For Claude Code, Codex, Gemini CLI, OpenCode, Cursor, GitHub
      Copilot CLI, and QwenCode, confirm current ACP support (native flag vs.
      adapter package vs. none) against each agent's own docs/`--help`, and
      record the adapter command template + capability flags
      (`supports_resume`, permission modes) in a short findings note.
      Verify: findings note lists a concrete command (or "no known ACP path
      yet") for all seven agents.
      See `acp-adapter-findings.md`. Registry populated in
      `knot-agent-launch::acp_adapter` for the five agent types Knot
      already supports (claude, codex, opencode, gemini, copilot) - Cursor
      and QwenCode aren't Knot agent types yet.
- [x] 1.2 For at least one confirmed-available adapter, manually run it as a
      subprocess and exchange a raw `initialize` + `session/new` +
      `session/prompt` JSON-RPC handshake (e.g. via a scratch script or
      `nc`/`socat` against its stdio) to confirm the wire format matches the
      spec assumptions in design.md. Verify: a captured transcript of a
      successful handshake is attached to the findings note.
      Ran against a live `gemini --acp` using the actual `knot-acp` client
      (not a throwaway script) - see the transcript in
      `acp-adapter-findings.md`. Found and fixed three real bugs:
      `session/new`/`session/load` missing `mcpServers`, `SessionUpdate`
      parsing the wrong (un-enveloped) shape, and `InitializeResult`
      reading `capabilities` instead of the real `agentCapabilities` key.
      Re-verified live after each fix; a real model turn ("pong") now
      parses correctly end to end.

## 2. `knot-acp` crate: transport and protocol types

- [x] 2.1 Scaffold `crates/knot-acp` (lib crate, workspace member, deps:
      `knot-core`, `serde`, `serde_json`, `tokio`). Verify: `cargo build -p
      knot-acp` succeeds with an empty lib.
- [x] 2.2 Implement JSON-RPC 2.0 framing over a subprocess's stdio (spawn,
      write requests, read/parse both single messages and batched arrays,
      match responses to requests by id). Verify: unit test spawns a fake
      subprocess (e.g. `cat` echoing crafted JSON-RPC) and asserts correct
      request/response pairing.
- [x] 2.3 Add the `initialize` request/response types and capability
      negotiation, failing closed on an unsupported protocol version per the
      `acp-client` spec. Verify: unit tests for matching-version success and
      mismatched-version error.
- [x] 2.4 Add session lifecycle methods (`session/new`, `session/load`,
      `session/prompt`, `session/cancel`, close) with typed errors for
      "resume not supported" per capability flags. Verify: unit tests cover
      each method's request shape and the resume-unsupported error path.
- [x] 2.5 Add the `session/update` notification stream as an ordered channel
      distinguishing text delta, tool-call start/update/result, diff, and
      turn-end-with-stop-reason. Verify: unit test feeds a sequence of
      update notifications and asserts the channel yields them in order with
      correct variant decoding.
- [x] 2.6 Add `session/request_permission` handling: surface the request,
      block the agent-facing response until the caller answers, and
      auto-decline any request still pending when the session closes.
      Verify: unit tests for normal answer flow and the close-while-pending
      auto-decline.
- [x] 2.7 Add subprocess exit / broken-pipe / JSON-RPC-error handling that
      ends the session with a reported cause instead of panicking, resolving
      any pending prompt/permission futures with an error. Verify: unit test
      kills the fake subprocess mid-turn and asserts the pending future
      resolves with an error, not a panic or hang.

## 3. Adapter registry and launch integration

- [x] 3.1 Add an adapter registry to `crates/knot-agent-launch` keyed by
      agent type, populated from the spike's findings (task 1.1): adapter
      command template, `supports_resume`, permission-mode support. Verify:
      unit test looks up each of the seven agent types and asserts the
      expected registry entry (including "no adapter" for unconfirmed
      types).
- [x] 3.2 Branch the launch path: Panel-mode agent with a registered adapter
      spawns the adapter and connects via `knot-acp`; Terminal mode or no
      adapter uses the existing terminal command path unchanged. Verify:
      existing `agent-launch-command` tests still pass unmodified, plus a
      new test asserting a Panel-mode, adapter-registered agent produces an
      adapter spawn instead of a terminal command.
- [x] 3.3 Thread MCP configuration and inline-registration-equivalent data
      into the adapter's own configuration mechanism (args or ACP session
      config) per agent type, matching the terminal path's MCP-enabled and
      MCP-disabled behavior. Verify: unit test for one adapter compares its
      effective MCP config in Panel mode against the terminal path's for the
      same settings.

## 4. Agent model and lifecycle

- [x] 4.1 Add `view_mode: ViewMode` (Panel | Terminal, default Terminal) and
      `acp_session_id: Option<String>` to `Agent` (`knot-agents`), threading
      through `convert.rs` persistence per the `agent-lifecycle` delta.
      Verify: `cargo test -p knot-agents` passes, including a new
      round-trip test for the two fields through save/load.
- [x] 4.2 Implement ACP session id resolution on layout restore (`session/
      load` attempt, fallback to fresh session on failure) and restart
      (close + clear ACP session id, same as existing session id clearing).
      Verify: unit tests for successful load, failed load falling back
      silently, and restart clearing both session ids.
      Note: `AgentStore::apply_acp_session_outcomes` covers the
      data-layer half (applying a precomputed load outcome per agent, and
      `restart` clearing `acp_session_id`) - the actual async `session/load`
      subprocess call and live-connection teardown belong to the runtime
      layer that owns a running `AcpClient` (`crates/knot`, not yet wired to
      any agent instance; that wiring lands with the panel UI in section 6).
- [x] 4.3 Implement the view-mode switch: starting/stopping the ACP
      connection without disturbing the underlying terminal process, per
      the `acp-panel-ui` "Switch to Terminal mid-turn" scenario. Verify:
      integration test toggles mode mid-turn and asserts the terminal
      process is untouched and the ACP turn's remaining updates still land.

## 5. Activity detection integration

- [x] 5.1 Add the `acp-updates` tracking-set variant to `knot-activity` and
      route Panel-mode agents to it exclusively (no `user-input`/
      `terminal-output` tracking), per the `activity-detection` delta.
      Verify: unit test asserts a Panel-mode agent's status is unaffected by
      simulated terminal output/keystrokes.
- [x] 5.2 Wire turn-start -> Working, turn-end-with-no-pending-permission ->
      Idle, permission-request -> Awaiting input, and ACP session error ->
      Error, with no idle timer or input-protection guard involved. Verify:
      unit tests for each of the four transitions driven by synthetic ACP
      events.

## 6. Panel UI

- [x] 6.1 Add a panel view module in `crates/knot` (sibling to
      `terminal_view.rs`) that folds a session's update stream into
      renderable state: message list with streaming text accumulation,
      tool-call cards, and pending permission state. Verify: a snapshot/unit
      test builds panel state from a scripted update sequence and asserts
      the resulting message/tool-call list matches expectations.
- [x] 6.2 Render tool-call cards by ACP `kind` (execute, read, edit, etc.),
      with a generic input/output fallback for unknown kinds, and render
      edit-kind results as an added/removed diff view. Verify: manual check
      against a live adapter session (from task 1.2) showing at least one
      command execution and one file edit rendered correctly.
      Code written (`panel_view.rs`) and wired in; compiles, clippy-clean,
      the app launches without panicking. The "manual check" itself -
      actually seeing a command execution and a file edit render - is not
      done: needs a human running the app with a live session, no visual
      output is available in this environment.
- [x] 6.3 Render pending permission requests inline with actionable
      allow/deny controls, blocking further prompt submission until
      answered, and send the chosen decision back through `knot-acp`.
      Verify: manual check with a live adapter that a deny decision is
      actually delivered (adapter's next behavior reflects the denial).
      Code written and wired in (`AcpSession::answer_permission`,
      `PanelSessionHandle::answer_permission`, the panel's allow/deny
      buttons). Same caveat as 6.2 - the actual "does the adapter's next
      behavior reflect the denial" manual check needs a human.
- [x] 6.4 Add the per-agent Panel/Terminal view-mode toggle to the agent
      header, following `knot-ui-conventions.md` (icon+tooltip button
      style, consistent with existing header controls), showing the toggle
      only for agent types with a registered adapter. Verify: manual check
      that agent types without an adapter show no toggle and always render
      the terminal.
      Wired via `SelectedAgentHeader.has_acp_adapter`
      (`knot_agent_launch::acp_adapter(...).is_some()`) gating the
      toggle's visibility, and `WorkspaceWindow::toggle_view_mode`. Same
      manual-check caveat as 6.2/6.3.

## 7. End-to-end verification

- [x] 7.1 Run `make rust` (fmt + clippy + test + build) across the
      workspace and confirm it passes with the new crate and modules
      included. Verify: command exits 0.
- [x] 7.2 Manually run one full session with a confirmed-working adapter
      (from task 1.1/1.2) end to end: create a Panel-mode agent, send a
      prompt, observe streaming text and at least one tool call, approve or
      deny a permission request if the agent triggers one, then switch to
      Terminal mode and confirm a real shell opens in the same folder.
      Verify: written confirmation (or short screen recording) that each
      step behaved as described.
      Confirmed by hand: streaming text, a tool call, and a permission
      prompt all rendered and behaved correctly.
- [x] 7.3 Confirm existing terminal-only agent types (no adapter) are
      unaffected: launch, activity detection, and history behave exactly as
      before this change. Verify: existing test suites for
      `agent-launch-command`, `agent-lifecycle`, and `activity-detection`
      pass unmodified for non-adapter agent types.
      Note: every pre-existing test in these three suites still passes
      unmodified in assertions (two call sites needed a new required
      argument - `tracking_for`'s `ViewMode`, `plan_launch`'s adapter
      param - but no existing behavior/assertion changed). Since
      `acp_adapter` returns `None` for every agent type (no adapter
      confirmed yet), every agent is still on the terminal path exactly as
      before.

## Post-merge hand-testing fixes (beyond the original task list)

Manual end-to-end testing (task 7.2, by the user) against real `gemini`
and `claude-agent-acp` adapters surfaced several real bugs, all fixed and
committed on `104-acp-agent-panel-ui`:

- `gemini` needs `--skip-trust` (adapter registry) or it blocks on an
  interactive trust prompt the ACP client can't answer.
- `AcpSession::start` had no timeout, so that hang was silent - added a
  20s connect timeout (`AcpError::Timeout`).
- Adapter subprocess stderr was discarded (`Stdio::null()`) - now piped
  and forwarded, prefixed with the adapter's command name.
- No prompt input existed at all originally - added (single-line input +
  Send button), which surfaced that ACP's `session/prompt` response
  doesn't resolve until the full turn completes (by design), so the UI
  redraw poll loop needed to keep working off the background
  event-drain task's dirty flag, not the prompt future's completion.
- `AdapterConfig` gained an `install: Option<InstallMethod>` field;
  `claude`'s missing adapter binary is now auto-installed via npm on
  first use, silently (design.md decision 6 - supersedes the original
  "don't invoke a package manager" framing of the non-goal, per explicit
  product direction). Verified against the real npm-installed binary.
- The user's own sent prompts weren't recorded anywhere (ACP's update
  stream only ever carries the *agent's* output) - added
  `PanelMessage::User`, recorded explicitly by
  `PanelSessionHandle::record_user_message` when a prompt is sent,
  rendered distinctly from assistant text.

### Known open gap: rich prompt content

The prompt input is currently text-only - `AcpClient::session_prompt`
sends a single `{"type":"text"}` content block. ACP's `prompt` field is
actually an array of content blocks (text, image, `resource_link`,
etc.), so file/image attachments are a real, spec-supported feature,
not a hack. The user asked for this (file attachment via the existing
`cx.prompt_for_paths` pattern used elsewhere in `main.rs`, e.g.
`choose_folder`) - **not yet implemented**, ran out of session budget
before starting. Design sketch handed off below for whoever picks this
up:

1. `knot-acp::AcpClient`: add `session_prompt_with_content(&self,
   session_id: &str, content: Vec<serde_json::Value>) -> Result<()>`,
   and refactor the existing `session_prompt(&self, session_id, text)`
   to call it with a single text block (no behavior change, just shared
   plumbing).
2. `knot-terminal::AcpSession`: add `prompt_with_attachments(&self,
   text: &str, file_paths: &[PathBuf]) -> AcpResult<()>` that builds a
   text block (if `text` is non-empty) plus one `resource_link` block
   per path (`{"type": "resource_link", "uri": "file://<path>", "name":
   <filename>}`), then calls `session_prompt_with_content`.
3. `WorkspaceWindow`: add `panel_prompt_attachments: BTreeMap<Uuid,
   Vec<PathBuf>>` (pending attachments per agent, cleared on send). An
   "Attach" button next to Send opens `cx.prompt_for_paths` (`files:
   true, directories: false, multiple: true`) and appends chosen paths;
   render a small chip row above the input listing attached filenames
   with a remove control. `send_panel_prompt` uses
   `prompt_with_attachments` instead of `prompt` when attachments are
   pending, and should probably fold the attachment names into the
   recorded `PanelMessage::User` text (e.g. append "📎 <filename>" per
   attachment) so they show in the conversation history too.
4. Unit-test `session_prompt_with_content`'s request shape and
   `prompt_with_attachments`'s content-block assembly directly (no live
   adapter needed for that part); the file-picker wiring itself can't be
   unit-tested and needs the same manual-check treatment as 6.2-6.4.
