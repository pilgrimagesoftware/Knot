## Why

Terminal-mode launch was kept as a permanent fallback when ACP support was
added (`acp-agent-panel-ui`), and every non-shell agent type still has a
Terminal/Panel view-mode toggle. In practice the app is only used in Panel
mode for real agents now, and the Terminal launch path has bit-rotted for
that use: MCP wiring is broken for ACP-launched agents (the `AdapterLaunch`
path reuses `mcp_arguments`, a shell-syntax string built for the terminal
command, and passes it as one raw `argv` element to a subprocess that never
parses it — so no ACP-launched agent has ever actually had a working knot
MCP connection, `set-status` included). Removing the terminal fallback for
primary agents forces the ACP path to be the one real launch path, and makes
fixing its MCP wiring a requirement rather than a someday cleanup.

## What Changes

- **BREAKING**: Non-shell agent types (`claude`, `codex`, `opencode`,
  `gemini`, `copilot`) SHALL launch exclusively through the ACP adapter path.
  The Terminal launch path (`build_agent_command`'s per-agent-type command
  string, the PTY/Grid terminal view) is no longer used for them.
- Remove the per-agent Panel/Terminal view-mode toggle for non-shell agents;
  Panel is the only mode. `ViewMode` no longer applies as a user choice for
  these types (still exists internally only insofar as the shell/companion
  path needs it, if at all).
- Shell companion agents (`agent_type == "shell"`, always companions per
  `agent-lifecycle`) keep the existing PTY/Grid terminal path unchanged -
  they have no ACP equivalent (a bare shell doesn't speak ACP).
- Fix MCP wiring for ACP-launched agents: replace the reused
  `mcp_arguments()` CLI-string hack with real MCP server configuration
  passed through the ACP protocol's own session-scoped mechanism
  (`session/new`'s MCP server params), so `register-agent`, `set-status`,
  and every other knot tool actually reach ACP-launched agents.
- An agent type with no registered ACP adapter (`knot_agent_launch::acp_adapter`
  returns `None`) and that isn't `shell` has no remaining launch path; this
  proposal doesn't add new adapters, so any such type becomes uncreatable
  going forward (currently every non-shell type Knot exposes has an adapter,
  so this has no practical effect today).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-launch-command`: the terminal command-building requirements now
  apply only to shell agents; non-shell agents launch via the ACP adapter
  path with MCP configuration delivered through the ACP protocol instead of
  reused CLI-argument strings.
- `agent-lifecycle`: drops the per-agent Terminal/Panel view-mode choice for
  non-shell agents (Panel only); shell/companion agents keep Terminal.

## Impact

- `crates/knot-agent-launch`: `build_agent_command`/`plan_launch` narrow to
  shell-only terminal commands; MCP config moves out of `mcp_arguments` and
  into a new ACP-session-params builder.
- `crates/knot-acp` / `crates/knot-terminal/src/acp_session`: `session/new`
  (and `session/load`) gain MCP server params; drop the
  `command.arg(&launch.mcp_config)` CLI-arg hack.
- `crates/knot/src/workspace_window`: remove the view-mode toggle button and
  `ViewMode` selection for non-shell agents; `ensure_session`/`dispatch_key`
  and related PTY plumbing become shell-only.
- `crates/knot/src/dashboard.rs`, `crates/knot/src/settings_window`: drop any
  Terminal-vs-Panel display/setting surfaced for non-shell agents.
- `openspec/changes/acp-agent-panel-ui` (still unarchived): its "terminal
  stays available, never removed" framing is superseded by this change for
  non-shell agents.
