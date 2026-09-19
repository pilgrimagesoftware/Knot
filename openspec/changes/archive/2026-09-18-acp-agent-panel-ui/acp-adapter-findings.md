# ACP Agent Support Research — September 2026

Research findings for Agent Client Protocol (ACP) support across seven coding-agent CLIs. Each agent is assessed for native ACP support, adapter requirements, launch commands, session resume, and permission mode capabilities.

## 1. Claude Code

**Status:** Adapter package required

**Command:** `claude-agent-acp` (after npm installation)

**Package Details:**
- Package name: `@agentclientprotocol/claude-agent-acp`
- Version: 0.76.0 (current as of September 2026)
- Installation: `npm install @agentclientprotocol/claude-agent-acp`
- Previously named `@zed-industries/claude-code-acp` (renamed in 2026)
- Citation: [npm package](https://www.npmjs.com/package/@agentclientprotocol/claude-agent-acp), [GitHub repo](https://github.com/agentclientprotocol/claude-agent-acp)

**supports_resume:** Yes
- Implements `session/load` ACP capability
- Recent performance improvements (v1089 PR): restored resumed models, cold-loading 69-message session reduced from 20-29s to 1.9s
- Citation: [GitHub PR #1089](https://github.com/agentclientprotocol/claude-agent-acp/pull/1089)

**supports_permission_modes:** Yes
- Supports ACP permission extension with auto-approval
- Falls back to `acceptEdits` when model can't use permissions
- Citation: [npm package docs](https://www.npmjs.com/package/@agentclientprotocol/claude-agent-acp)

**Confidence:** high

---

## 2. Codex

**Status:** Adapter package required

**Command:** `codex-acp` (executable built from Rust source)

**Package Details:**
- Package repo: `cola-io/codex-acp`
- Language: Rust (2024 edition, rustc 1.91+)
- Installation: 
  - Clone: `git clone https://github.com/cola-io/codex-acp.git`
  - Build: `make release`
  - Output: `./target/release/codex-acp`
- Exposes internal MCP filesystem server (`acp_fs`) built with rmcp
- Citation: [GitHub repo](https://github.com/cola-io/codex-acp), [Releases](https://github.com/cola-io/codex-acp/releases)

**supports_resume:** Unknown
- No explicit mention of `session/load` in available documentation
- Session management supported with three modes (read-only, auto, full-access)
- Citation: [GitHub repo README](https://github.com/cola-io/codex-acp)

**supports_permission_modes:** Yes
- Implements three session access modes: read-only, auto, and full-access
- Dynamic slash command advertisement
- Citation: [GitHub repo](https://github.com/cola-io/codex-acp)

**Confidence:** high

---

## 3. Gemini CLI

**Status:** Native ACP support

**Command:** `gemini --acp`

**Additional options:**
- Debug flag: `gemini --acp --debug`
- Citation: [Gemini CLI docs](https://geminicli.com/docs/cli/acp-mode/)

**supports_resume:** Yes
- Session management with persistence
- Permission state saved across sessions
- Configuration persists across IDE and terminal interactions
- Citation: [PR #23818 on google-gemini/gemini-cli](https://github.com/google-gemini/gemini-cli/pull/23818), [IDE integration docs](https://geminicli.com/docs/ide-integration/)

**supports_permission_modes:** Yes
- Permission request flow for file attachments and operations
- Options: "Allow for this session", "Allow and Save Permanent", "Reject"
- Supports temporary read-only access to files outside workspace
- Citation: [PR #23680](https://github.com/google-gemini/gemini-cli/pull/23680), [ACP docs](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/acp-mode.md)

**Confidence:** high

---

## 4. OpenCode

**Status:** Native ACP support

**Command:** `opencode acp`

**Additional notes:**
- Starts as ACP-compatible subprocess
- Communicates via JSON-RPC over stdio
- All terminal features supported except `/undo` and `/redo`
- Supports: file operations, terminal commands, MCP servers, custom tools, project rules (AGENTS.md)
- Citation: [OpenCode docs](https://opencode.ai/docs/acp/), [CLI docs](https://opencode.ai/docs/cli/)

**supports_resume:** Yes
- Standard ACP session capabilities
- Session persistence supported
- Citation: [OpenCode ACP docs](https://opencode.ai/docs/acp/)

**supports_permission_modes:** Yes
- Agents and permissions system documented
- Built-in tool approval workflows
- Citation: [OpenCode docs](https://opencode.ai/docs/acp/)

**Confidence:** high

---

## 5. Cursor

**Status:** Native ACP support

**Command:** `agent acp`

**Alternative forms:**
- `cursor-agent acp` (earlier naming)
- With auth: `agent --api-key "$CURSOR_API_KEY" acp`
- With endpoint: `agent -e https://api2.cursor.sh acp`
- Citation: [Cursor docs](https://cursor.com/docs/cli/acp)

**supports_resume:** Yes
- Implements `session/load` for resuming previous sessions
- Implements `session/new` for creating fresh sessions
- Uses newline-delimited JSON-RPC 2.0 over stdio
- Citation: [Cursor ACP docs](https://cursor.com/docs/cli/acp)

**supports_permission_modes:** Yes
- Three mode system: agent mode (full access), plan mode (read-only), ask mode (read-only Q&A)
- Permission request system via `session/request_permission`
- Response options: `allow-once`, `allow-always`, `reject-once`
- Citation: [Cursor ACP docs](https://cursor.com/docs/cli/acp)

**Confidence:** high

---

## 6. GitHub Copilot CLI

**Status:** Native ACP support

**Command:** `copilot --acp [transport-options] [session-options]`

**Transport options (required, choose one):**
- `--stdio` (default, NDJSON over standard input/output)
- `--port PORT` (TCP mode on specified port, e.g., `--port 3000`)

**Session configuration options:**
- `--available-tools=TOOL,...` (comma-separated tool whitelist)
- `--excluded-tools=TOOL,...` (comma-separated tool blacklist)
- `--effort=LEVEL` or `--reasoning-effort=LEVEL` (low, medium, high, xhigh, max)

**Example:** `copilot --acp --port 3000 --effort=max --available-tools="bash,view"`

**Status note:** Public preview launched January 28, 2026
- Citation: [GitHub Changelog](https://github.blog/changelog/2026-01-28-acp-support-in-copilot-cli-is-now-in-public-preview/), [GitHub Docs](https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server)

**supports_resume:** Yes
- Sessions are long-lived
- Client can initialize once, open a session, then issue prompt turns
- Server streams `session/update` notifications before turn resolution
- Citation: [GitHub Docs](https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server)

**supports_permission_modes:** Yes
- Tool availability controlled via `--available-tools` and `--excluded-tools`
- Initial reasoning effort level set via `--effort` or `--reasoning-effort`
- Each session created under these server-wide settings
- Citation: [GitHub Docs](https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server)

**Confidence:** high

---

## 7. QwenCode (Qwen)

**Status:** Native ACP support via daemon mode OR adapter package available

**Native command:** `qwen serve` (HTTP+SSE ACP server)

**Key options:**
- `--hostname <addr>` (bind interface, default: `127.0.0.1`)
- `--port <n>` (TCP port, default: `4170`)
- `--token <str>` (bearer token for API)
- `--require-auth` (mandate authentication on loopback)
- `--web` / `--no-web` (enable/disable Web Shell, default: enabled)
- `--open` (auto-launch Web Shell in browser)
- `--max-sessions <n>` (concurrent session cap, default: 32)

**Adapter package (alternative):**
- Package repo: `menhazalam/acp-qwen-code`
- Installation: `npm install @acp-providers/qwen-code` or build from GitHub
- Citation: [GitHub adapter repo](https://github.com/menhazalam/acp-qwen-code)

**Daemon mode notes:**
- Exposes HTTP API with SSE streaming
- Supports shared agent session across multiple clients
- Reconnect-safe event replay via `Last-Event-ID`
- Optional MCP server and skill runtime management
- Citation: [Qwen Code daemon docs](https://qwenlm.github.io/qwen-code-docs/en/users/qwen-serve/)

**supports_resume:** Yes
- Multi-client session support via daemon
- Reconnect-safe design
- Session state persisted across client disconnects
- Citation: [Qwen Code daemon docs](https://qwenlm.github.io/qwen-code-docs/en/users/qwen-serve/)

**supports_permission_modes:** Yes
- Permission request coordination in daemon mode
- Multi-workspace hosting with isolated permissions per workspace
- `--memory-project-scope` controls memory partitioning (workspace or git-root)
- Citation: [Qwen Code daemon docs](https://qwenlm.github.io/qwen-code-docs/en/users/qwen-serve/)

**Confidence:** high

---

## Summary Table

| Agent | ACP Path | Launch Command | Resume | Permissions |
|-------|----------|-----------------|--------|-------------|
| Claude Code | Adapter (npm) | `claude-agent-acp` | Yes | Yes |
| Codex | Adapter (Rust build) | `codex-acp` | Unknown | Yes |
| Gemini CLI | Native | `gemini --acp` | Yes | Yes |
| OpenCode | Native | `opencode acp` | Yes | Yes |
| Cursor | Native | `agent acp` | Yes | Yes |
| GitHub Copilot CLI | Native | `copilot --acp` | Yes | Yes |
| QwenCode | Native (daemon) | `qwen serve` | Yes | Yes |

---

## Notes

- **ACP Specification:** All findings reflect Agent Client Protocol v0.11.0+ (March 4, 2026). The protocol uses JSON-RPC 2.0 over stdio (or TCP for Copilot/Qwen).
- **Confidence Levels:** "high" = findings from official docs, GitHub repos, or npm packages as of September 2026; "medium" = some documentation gaps but corroborated by multiple sources; "unknown" = documentation does not provide clear confirmation.
- **Permission Modes:** ACP specification includes permission request mechanism (`session/request_permission`). Agents advertise available modes affecting system prompts, tool availability, and permission behavior.
- **Session Resume:** ACP spec defines `session/load` capability. Only Codex documentation is unclear; others explicitly support or demonstrate session persistence.

## Task 1.2: live handshake transcript (`gemini --acp`)

Verified against locally-installed `gemini` (v0.46.0) outside the sandbox (real
network + `GEMINI_API_KEY`), using the actual `knot-acp` client
(`AcpClient::connect` + `session_new` + `session_prompt`), not a scratch
script — `--skip-trust` was needed since the working directory isn't a
gemini-trusted folder.

```
capabilities: AgentCapabilities { supports_resume: false, permission_modes: [] }
session_id: 15246566-42e9-4736-8256-6cb0a04849d9
update: Unknown { raw: {"sessionId": "...", "update": {"sessionUpdate": "available_commands_update", ...}} }
update: TextDelta { text: "pong" }
```

The subprocess answered `initialize` (protocolVersion 1, `loadSession: true`
in its agentCapabilities), `session/new`, and streamed real
`session/update` notifications for a live prompt turn ("Reply with exactly
the word: pong" → the model actually replied "pong").

This surfaced three real bugs in `knot-acp`, all fixed in this pass and
re-verified live afterward:

1. **`session/new`/`session/load` were missing `mcpServers`.** Gemini's
   adapter rejects the request without it (`invalid_type: expected array,
   received undefined` at `mcpServers`). Fixed: both now send `"mcpServers":
   []` when no MCP config is threaded through yet.
2. **`SessionUpdate::from_params` read the wrong shape.** The real
   `session/update` notification nests the typed update under an `"update"`
   envelope (`{"sessionId": ..., "update": {"sessionUpdate": ...}}`), and
   text content is a content block (`content.text`), not a flat `text`
   field - both `text_delta` and `agent_message_chunk` updates were
   silently falling into `Unknown` before this fix.
3. **`InitializeResult.capabilities` read the wrong JSON key.** The real
   field is `"agentCapabilities"`, not `"capabilities"` - the mismatch was
   silent (`#[serde(default)]` filled in `false`/`[]` instead of erroring),
   so `AgentCapabilities::supports_resume` read `false` for gemini even
   though its `initialize` response reports `"loadSession":true`. Fixed by
   renaming the field; re-verified live - `supports_resume` now reads
   `true`.

Re-running the same live prompt after all three fixes: `capabilities:
AgentCapabilities { supports_resume: true, permission_modes: [] }`, then a
correctly-parsed `TextDelta { text: "pong" }` for the model's real reply.
(`permission_modes` stays empty - gemini's `initialize` response doesn't
send a `permissionModes` key at all, so this is a genuinely absent
capability, not a parsing bug; not chased further this pass.) Also observed
an `agent_thought_chunk` update kind (gemini's reasoning trace) that
`SessionUpdate` has no variant for yet and files under `Unknown` -
harmless today, but relevant when task 6.x builds the "thought/reasoning
collapse" panel UI element design.md calls for.

