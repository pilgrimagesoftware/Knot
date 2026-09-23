# Design

## Context

See `proposal.md` — Why. The constraints that shape the approach:

- **ACP tells Knot nothing.** `session/new`'s `mcpServers` is written once at
  session start (`crates/knot-acp/src/client/mod.rs:187-236`) and never read
  back; `SessionUpdate::from_params`
  (`crates/knot-acp/src/protocol/mod.rs:224-266`) has no kind carrying MCP
  state. There is no request to add, either — the protocol has no such
  method. Any state Knot shows has to come from outside the session.
- **The agent's own CLI is the only source of truth, and it is not a
  library.** `claude mcp list` has no `--json`; it prints lines for humans
  and health-checks as it goes. `opencode mcp list` and `gemini mcp list`
  exist with shapes of their own. `codex` and `copilot` were not available
  on the machine this was scoped on.
- **Knot already has the parts.** `knot-forge` is the same shape of problem
  solved once (run a foreign CLI, classify its failure, surface it in a
  pane). `crates/knot/src/mcp_status.rs` already models Knot's own server's
  state. `knot-terminal`'s `send_text`/`send_return` already inject into a
  live PTY (`crates/knot-terminal/src/lib.rs:136-141`, used by
  `on_inject_registration` at `:183-190`). `acp-panel-ui` already guarantees
  a plain terminal can be opened for any agent's working directory.
- **House rules that bind this change.** No I/O on the render path; an
  off-thread result reaches a frame only through `repaint_poll_tick`'s chain
  (`crates/knot/src/workspace_window/repaint.rs:57`); closed vocabularies are
  enums, not strings with a default arm; `mod.rs` declares and does not
  implement; no `.rs` over 700 lines.

## Goals / Non-Goals

**Goals:**

- One place in a Panel-mode agent's pane that answers "which MCP server needs
  attention, and what do I run".
- A probe that is honest about what it is: a separate process asking the same
  configuration, timestamped, never dressed up as session state.
- Per-agent-type knowledge in exactly one place, with a test that fails when
  a known type has no row.
- Every state the real tooling reports survives the trip, including the two
  that are not failures.

**Non-Goals** (design-level, beyond the proposal's):

- Reimplementing any agent's MCP configuration resolution. Scope precedence,
  plugin-contributed servers, project approval — that is the CLI's logic and
  Knot does not copy it.
- A persistent cache. Probe results live for the window's lifetime and are
  not written to settings or to disk.
- Any change to how Knot's own MCP server is configured, started or
  supervised.

## Decisions

### A new crate, `knot-mcp-probe`

The probe runs a subprocess and parses text. That is testable without GPUI,
without tokio's reactor and without a window — exactly the split `knot-forge`
and `knot-git` already make. Putting it in `crates/knot` would bury the
parser behind the UI and make its fixtures awkward to run.

*Alternatives:* a module in `knot-agents` (which is the roster and store, not
a process runner); a module in `knot-mcp` (which is Knot's own server — the
name would imply the two are related, and they are not).

The name deliberately does not shorten to `knot-mcp-*` ambiguity:
`knot-mcp` serves, `knot-mcp-tools` catalogs, `knot-mcp-probe` interrogates
somebody else's.

### Probe by running the CLI, not by reading config files

Reading `~/.claude.json`, `.mcp.json` and `~/.codex/config.toml` would give
the list without a round-trip. It would not give the state, and state is the
entire question — "is this server connected" is not in any config file.

It is also more code than it looks: the observed output includes servers
contributed by plugins and servers disabled per project, neither of which
exists as a plain entry in a user config file. Composing that list is the
CLI's job, done correctly, already.

*Cost:* a subprocess and, for HTTP servers, network latency. Bounded by the
timeout below and by never putting the probe on a tick.

### Run it in the agent's own environment

Project-scoped configuration resolves relative to the working directory, and
`CLAUDE_CONFIG_DIR` and friends change which user config is in play. The
probe therefore runs with the same working directory and the same environment
Knot used to launch that agent, so it resolves the same servers the agent
resolved. A probe run from Knot's own cwd would silently answer a different
question.

Stdin is closed so a CLI that would prompt fails fast rather than hanging to
the timeout.

### Parse from the right, anchored on the state glyph

Observed `claude mcp list` output:

```
<name>: <target> - <glyph> <state text>
```

`<target>` is unbounded and contains ` - ` freely — a real stdio entry on the
scoping machine was a ~1.5 KB `node -e` program. Splitting on the first (or
any) ` - ` is wrong. The parse finds the **last** ` - ` that is followed by a
recognized state glyph, and takes the name as everything before the first
`": "`.

State glyphs map to the spec's vocabulary:

| Observed | State |
| --- | --- |
| `✔ Connected` | Connected |
| `✘ Failed to connect — <error>` | Failed (error retained) |
| `⊘ Disabled for this project` | Disabled |
| `⏸ Pending approval` | PendingApproval |
| needs-auth wording | NeedsAuthentication |
| anything else | Unknown |

A line that does not parse becomes an `Unknown` row rather than being
dropped, so a format change degrades to "I can see a server and cannot tell
you about it" rather than to a shorter list. Output with no parsable line at
all is a probe failure, reported with its first line of output, not an empty
inventory. Captured fixtures are the contract; there is no `--json` to lean
on and the format can change under us.

### Two columns on the `agent_type` roster

`knot_core::agent_type` already carries per-type columns and has the test
that fails when a known type has none. Two are added:

- the read-only list command, and
- how that type manages a server, as an enum rather than an `Option<String>`:
  a command addressing one named server (`opencode mcp auth <name>`), an
  interactive flow the user is dropped into (Claude Code: run the CLI, send
  `/mcp`), or nothing.

Modelling those two cases as one nullable string would force every call site
to re-derive which it is. The enum makes "this type has no per-server
command, drop into its UI" a branch the compiler checks.

A type whose shape is not yet known ships with no command and reads as
"cannot determine", which the spec already requires to be distinct from "has
none".

### Delegation spawns a transient terminal, not the adapter's

The ACP adapter's subprocess is not an interactive CLI and must not be typed
into. The action uses the terminal `acp-panel-ui` already guarantees — a
plain shell in the agent's working directory, independent of the ACP session
— and injects through `send_text`/`send_return`, the path
`on_inject_registration` already uses. The terminal's exit is the re-probe
trigger.

**The injected command is built from the roster, never from probe output** —
with one exception, the server name for a per-server command. That name comes
from parsed text and lands on a shell command line, so it is shell-quoted and
rejected if it does not match a conservative pattern; a server whose name
fails that check gets the interactive flow instead. This is the only place in
the change where foreign text reaches a command line, and it is the one
thing in it worth reviewing twice.

### Probe scheduling: event-driven, one in flight, landed through the poll tick

Per agent, a slot holding `NotProbed | InFlight | Done { at, outcome }`. A
probe is started on the section becoming visible for a running agent, on
refresh, and on a delegated terminal's exit — not on an interval. An MCP
health check crosses the network; a 3-second tick per shown agent would be a
different change with a different cost section.

`spawn_blocking` runs it; the result sets the slot and a changed flag.
`repaint_poll_tick` gains one branch that takes the flag and notifies. Per
`.claude/rules/rust-structure.md`, the clearing read lives only there — no
other reader calls it, because a read on a path that discards the result is
how this has broken four times before.

A request arriving while a probe is in flight is dropped, not queued: the
result of the in-flight probe is the answer to both.

### Merging Knot's own row on endpoint, not name

`MCP_SERVER_NAME` is the name Knot registers under; a user who ran the
settings window's install command chose their own. Identity is the normalized
endpoint compared against `mcp_url_for_port(settings.mcp_server_port)`
(`crates/knot-agent-launch/src/builders.rs:12-26`). The merged row shows the
state Knot's supervisor reports — it is authoritative and instant — and
notes the independent registration, because that copy survives Knot and the
user may want it gone.

### Rows show a bounded identifier

HTTP: transport and host. Stdio: the program's basename. The full target is
behind a copy action. The 1.5 KB `node -e` entry is the reason this is a
requirement and not a nicety — rendering it would blow up a row's height on
the virtualized list.

## Risks / Trade-offs

- **`claude mcp list`'s format changes and every row reads Unknown** → the
  parse degrades per-line rather than wholesale, fixtures pin the current
  shape, and an entirely unparsable output is a reported failure rather than
  a silent empty list. Accepted: there is no stable machine interface to use
  instead.
- **A probe is slow or hangs** → hard timeout in `knot-mcp-probe::consts`,
  child killed on expiry, stdin closed so nothing can wait on input, one
  probe per agent, never on a tick. The section reports the timeout instead
  of sitting in the in-progress state.
- **A server name from parsed output reaches a shell command line** →
  shell-quoted and pattern-checked; a name that fails falls back to the
  interactive flow, which names no server.
- **The probe answers a different question than the session** (it resolves
  configuration fresh rather than reflecting the running session's
  connections) → stated in the UI by timestamping the rows, and in the
  proposal's scoping. This is a genuine limitation, not a defect to fix
  later: ACP offers nothing better.
- **Probing costs a subprocess and network time the user did not ask for** →
  gated on the section being visible for a running Panel-mode agent, and
  event-driven. A user who never opens the section never pays.
- **Per-type coverage is uneven at first** — only the types whose output
  shape has been captured can be probed → the spec already requires "cannot
  determine" to be distinct from "none", so an uncovered type is honest
  rather than wrong, and adding one later is a roster row plus a fixture.

## Migration Plan

Purely additive. No persisted state, no settings key, no change to how any
agent is launched or how Knot's own server runs. The section is new surface
in a pane; nothing existing changes shape. Rollback is reverting the change —
no data is written that would outlive it.

## Open Questions

- **The exact output shapes of `codex mcp list`, `gemini mcp list`,
  `opencode mcp list` and Copilot's equivalent.** Neither `codex` nor
  `copilot` was installed on the machine this was scoped on. Deferrable by
  construction: each is one roster row plus one fixture, and until it exists
  that type reports "cannot determine" — which the spec already defines.
  Claude Code's shape is captured and is enough to ship the capability.
