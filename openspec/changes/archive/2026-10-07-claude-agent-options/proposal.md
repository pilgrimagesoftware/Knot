# Proposal

## Why

Options set for Claude in Settings - Coding - Agent Options never reach the
agent (#501). A user who sets `--dangerously-skip-permissions --remote-control`
gets a `claude` process with neither. The terminal builder that used to append
these options to the command line was removed by `acp-only-agent-launch`, and
nothing on the ACP path took its place: `agent_options` is saved and shown, and
no launch path reads it.

## What Changes

- A Claude agent's options are passed to `claude-agent-acp` as
  `_meta.claudeCode.options.extraArgs` on `session/new` and `session/load`, the
  adapter's own channel for extra CLI flags. The adapter writes them onto the
  `claude` process it spawns.
- `--dangerously-skip-permissions` and `--permission-mode <mode>` become the
  mode the session is moved to once it opens, on every launch, since the
  adapter always passes its own `--permission-mode`.
- Flags the adapter passes itself to drive the session (`--output-format`,
  `--resume`, ...), short flags and stray words are not forwarded; Knot logs
  what it dropped.
- When an adapter refuses a session with an error that carries details, the
  error Knot shows includes them, so a mistyped option names itself instead of
  reading "Internal error".

## Non-Goals

- Options for Codex, OpenCode, Gemini and Copilot. Each adapter takes them a
  different way, none confirmed yet; they are logged as not forwarded.
  Follow-up issue.
- Changing the Coding tab or how options are stored.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-launch-command`: adds "User options for an ACP-launched agent".

## Impact

- `knot-agent-launch`: new `agent_options` module (`adapter_options`), constants.
- `knot-acp`: `AcpClient::with_session_meta`; RPC errors carry `data.details`.
- `knot-terminal`: `AcpSession::start` takes a `SessionTarget`.
- `knot`: `ConnectRequest` carries the session `_meta` and the options' mode;
  `ensure_panel_session` reads `agent_options`.
