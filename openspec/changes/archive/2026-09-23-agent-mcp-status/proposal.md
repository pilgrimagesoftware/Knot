# Proposal

## Why

An agent in Panel mode tells the user that one of its MCP servers is
disconnected, needs re-authentication, or failed to start. There is nothing
the user can do about it from Knot. The agent's own remedy — Claude Code's
`/mcp`, Codex's `codex mcp`, Gemini's `gemini mcp` — is an interactive
command in the agent's own terminal UI, and a Panel-mode agent has no
terminal the user types into. The panel's slash lookup does not help: it
completes text and the agent interprets it, and `/mcp` is not text the agent
interprets — it is a client-side command of a CLI that is not running in
Panel mode.

So the user either restarts the agent and hopes, or leaves Knot to fix it and
comes back. The state is visible and unactionable, which is the worst of the
two.

Knot already solves the same shape of problem for a different integration:
`knot-forge` detects that `gh` is unauthenticated and the pull requests pane
says so (`ForgeAvailability::Unauthenticated`). MCP has no equivalent.

## What Changes

- **A Panel-mode agent's pane gains an MCP servers section**, collapsible,
  beside the processes section. Collapsed it names what needs attention;
  expanded it lists every server with its state.
- **The list has two sources, and says which is which.** Knot's own `knot`
  server comes from Knot's supervisor, which already tracks it
  (`McpServerStatus`). The agent's own servers come from running that agent
  type's read-only MCP list command in the agent's working directory.
- **Server state is a closed vocabulary** — connected, needs authentication,
  pending approval, disabled, failed, unknown — not a string with a default
  arm. The last four are all things the real tooling reports today: `claude
  mcp list` distinguishes `⊘ Disabled for this project`, `⏸ Pending
  approval` and `✘ Failed to connect — <error>`, and collapsing them into
  "broken" would tell the user to repair a server they themselves turned
  off.
- **Knot's own server is shown once.** A user who followed the settings
  window's install command has `knot` in the agent's own configuration too,
  so the probe reports it alongside the copy Knot injects. The row is merged,
  keyed on the endpoint rather than the name, and says that the agent
  configures it independently.
- **A server needing attention offers one action: hand the user the agent's
  own flow.** Knot spawns a plain terminal in the agent's working directory
  and types that agent type's MCP command into it. Where the type addresses
  one server directly — OpenCode has `opencode mcp auth <name>` — the action
  names the row's server; where it only has an interactive flow, as Claude
  Code does, the action lands the user in it. The OAuth browser round-trip,
  the device code, the server-specific prompt all happen where they already
  work. Knot re-probes when that terminal exits.
- **A type Knot cannot probe says so.** "No MCP command for this agent type"
  and "this agent has no MCP servers configured" are different answers, and
  the section never shows the second when it means the first.
- **The MCP command becomes a column on the `agent_type` roster**, next to
  the install command already there, with the roster's existing test that
  fails when a known type has no row.

### What this deliberately does not try to be

Knot cannot see the running agent's live MCP connections. ACP has no method
for it: `session/new`'s `mcpServers` is written once at session start and
never read back, and no `sessionUpdate` kind carries server state
(`crates/knot-acp/src/client/mod.rs:187`,
`crates/knot-acp/src/protocol/mod.rs:224`). The probe is therefore a separate
process asking the same configuration the same question — a fresh health
check against the same servers, not a window into the session. That is enough
to answer "which server needs attention and what do I run", which is the
question the user is stuck on, and it is not enough to be called live. The
section timestamps what it shows rather than implying otherwise.

### Cost

One subprocess per probe (`<cli> mcp list`), run off the render path on a
blocking task, gated the way the process sampler is gated: only for a
Panel-mode agent whose pane is shown and which is running. Probing is not on
a fast timer — it runs when the section first becomes visible, when the user
refreshes, and after a delegated action's terminal exits. An MCP health
check can take seconds and can hit the network, which is precisely why it is
not on a 3-second tick.

### Non-goals

- **Adding, editing or removing MCP servers from Knot.** That means writing
  `~/.claude.json`, `.mcp.json`, `~/.codex/config.toml` and whatever each
  agent adopts next — formats Knot does not own and cannot keep up with. The
  agent's own `mcp add` is one delegated action away.
- **Performing authentication inside Knot.** No OAuth flow, no token storage,
  no credentials in Knot's settings.
- **Terminal-mode agents.** `/mcp` already works there; it is a real PTY
  running the real CLI. The section is Panel-mode only.
- **A marker on the sidebar agent row.** Worth having, and a separate change
  against `agent-list-ui` — this one earns its keep by making the state
  reachable at all.
- **Changing the settings window's MCP tab.** That tab configures Knot's own
  server and is unaffected.
- **Parsing an agent's MCP output to guess at state.** The probe's exit status
  and structured output are the source; no scraping of the conversation.

## Capabilities

### New Capabilities

- `agent-mcp-status`: the MCP servers section in a Panel-mode agent's pane —
  what it lists, where each row's state comes from, when the probe runs, and
  the delegated action for a server needing attention.

### Modified Capabilities

None. The section is new surface in the agent pane, the way `agent-processes`
is; `acp-panel-ui`'s existing "Terminal remains available" requirement already
provides the plain shell the delegated action spawns.

## Impact

- **New crate `knot-mcp-probe`** — runs an agent type's MCP list command and
  parses it into a typed inventory. Runtime-agnostic, subprocess-based and
  unit-testable without GPUI, the way `knot-forge` and `knot-git` are. Named
  to keep it distinct from `knot-mcp` (Knot's own server) and
  `knot-mcp-tools` (the tools that server exposes). It parses human output:
  `claude mcp list` has no `--json`, so the parser is pinned to a format that
  can change under it, and its fixtures are the contract. Unrecognized output
  is `Unknown`, never silence.
- **`knot-core::agent_type`** — one new column, plus the roster test.
- **`crates/knot/src/workspace_window/`** — a new `mcp_panel/` module
  following `git_panel/`'s shape, a branch in `repaint_poll_tick`'s chain so
  probe results reach a frame, and the terminal-injection path that already
  exists in `knot-terminal` (`send_text`/`send_return`) for the delegated
  action.
- **`knot-core::l10n`** — new keys for every state, the empty case, the
  unprobeable case and the action labels.
