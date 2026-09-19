## Context

See `proposal.md` - Why for motivation. Two independent findings narrow this
to a precise fix, not a speculative one:

1. `AcpClient::session_new`/`session_load` (`crates/knot-acp/src/client/mod.rs`)
   hard-code `"mcpServers": []` on every request - the ACP protocol's actual,
   session-scoped MCP mechanism (confirmed real: a comment there notes Gemini
   CLI rejects `session/new` without the `mcpServers` key at all) is wired up
   but always sent empty. This is the real, correct integration point and it
   has never carried Knot's MCP server.
2. Separately, `AdapterLaunch.mcp_config` (`crates/knot-agent-launch`) reuses
   `mcp_arguments()` - a shell-syntax string built for the terminal command
   (e.g. `--mcp-config '{...}' --allowed-tools '...'`) - and
   `crates/knot-terminal/src/acp_session/mod.rs` passes that whole string as
   one literal `argv` element to the adapter subprocess via
   `command.arg(&launch.mcp_config)`. `std::process::Command` does no shell
   parsing, so even where this string is non-empty (claude, copilot) the
   subprocess receives one garbled argument, and for `opencode` it's always
   empty (no match arm in `mcp_arguments`, confirmed intentional by an
   existing test). This path never worked and has no ACP-protocol
   justification once (1) is fixed - it is removed, not repaired.

Non-shell agents currently have a Terminal/Panel view-mode toggle
(`WorkspaceViewMode`/per-agent `ViewMode`, wired through `plan_launch` in
`crates/knot-agent-launch`). Shell agents already only ever use the Terminal
path (no ACP adapter is registered for `"shell"`).

## Goals / Non-Goals

**Goals:**
- Make `session/new`/`session/load` pass the real knot MCP server config
  when MCP is enabled, so every ACP-launched agent type gets working MCP
  tools (including `set-status`) the same way.
- Remove the terminal launch path, the view-mode toggle, and the dead
  `AdapterLaunch.mcp_config` CLI-arg plumbing for non-shell agent types.
- Keep shell/companion agents unchanged (still Terminal/PTY, no MCP).

**Non-Goals:**
- Changing anything about how the panel UI renders (`panel_view`,
  `panel_state`) - out of scope; this is launch/wiring only.
- Adding ACP support for a currently-unsupported agent type.
- Reconciling this change with the still-unarchived `acp-agent-panel-ui`
  change's spec deltas - that's a pre-existing housekeeping gap; this change
  only touches the specs already promoted under `openspec/specs/`.

## Decisions

**MCP config shape sent to `session/new`/`session/load`**: an array with one
entry naming Knot's server and its HTTP URL, matching the shape the Gemini
CLI comment already implies the protocol expects, e.g.
`[{"name": "knot", "type": "http", "url": "<mcp_url>"}]`. Built from
`Settings::mcp_server_enabled`/`mcp_server_port` the same way
`mcp_arguments`'s `claude` arm already computes its URL - no new settings.
Alternative considered: keep per-adapter CLI flags as well (belt-and-braces).
Rejected - the protocol-level mechanism is universal across adapters and
already required by at least one of them; duplicating via broken CLI args
adds no coverage and keeps the buggy code path alive.

**Where the MCP params are threaded through**: `AcpClient::session_new`/
`session_load` gain an `mcp_url: Option<&str>` parameter (or equivalent),
supplied by `PanelSessionHandle::start` from `Settings`, rather than
threading it through `AdapterLaunch` (which is deleted along with the
terminal-arg path). Keeps the protocol-shape concern inside `knot-acp`
(agent-agnostic, per its own module doc) instead of leaking JSON construction
into `knot-agent-launch`.

**Removing `ViewMode` entirely vs. narrowing it to shell-only**: narrow, not
remove. `knot_core::ViewMode` (Panel/Terminal) stays as a type since shell
agents still need to be distinguished from a settings-persistence
perspective (the field is still read on layout restore), but non-shell
agents are no longer permitted to hold `Terminal`, and the UI toggle that
lets a user choose is removed for them. Enforced at the one place agents are
created/edited (`AgentStore`), not scattered `if agent_type != "shell"`
checks - see tasks.md.

**Deleting vs. deprecating the terminal-command build path**: delete
`build_agent_command`'s non-shell branches and the resume/registration/
persona helpers it alone used (`mcp_arguments`'s non-empty arms, inline
registration arguments, persona injection) rather than leaving them as dead
code, per the removed-requirements deltas in `agent-launch-command`. Nothing
else calls them once `plan_launch` only builds a shell terminal command or an
ACP launch.

## Risks / Trade-offs

- [Risk] Some ACP adapters may not actually support the `mcpServers` param
  the way Gemini's comment suggests, or may want a different shape per
  adapter → Mitigation: this is confirmed for Gemini today; if another
  adapter's handshake rejects the shape, that adapter's `AdapterConfig` gains
  an explicit override, not a global fallback - fix forward per-adapter, same
  as the existing `supports_resume`/`supports_permission_modes` flags.
- [Risk] Existing users relying on Terminal mode for a non-shell agent type
  lose that capability with no opt-out → Mitigation: explicitly accepted as
  **BREAKING** per the proposal; shell agents remain for real terminal need.
- [Risk] Removing `build_agent_command`'s non-shell branches touches code
  with existing test coverage (`agent-launch-command` spec scenarios) →
  Mitigation: the spec deltas already enumerate exactly which
  scenarios/tests are removed vs. kept; tasks.md updates the corresponding
  `#[cfg(test)]` modules in lockstep.

## Migration Plan

No data migration - `ViewMode` values already persisted as `Terminal` for a
non-shell agent (there shouldn't be any live ones today, since no adapter
existed for opencode/etc. to *not* use, but layout restore is defensive
regardless) are coerced to `Panel` on load for non-shell agent types, same
place `agent_type` already defaults on legacy records.

Rollback: revert the commit(s); no schema change to unwind.
