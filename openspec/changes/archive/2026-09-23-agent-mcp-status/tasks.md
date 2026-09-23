# Tasks

## 1. `knot-mcp-probe`: the typed inventory

- [x] 1.1 Create the crate with `thiserror`-based `ProbeError` and a crate `Result` alias, registered in the workspace and in `[workspace.dependencies]` where it is shared; verify `cargo build --workspace` picks it up and `make size-check` still passes
- [x] 1.2 Define `ServerState` as a closed enum — `Connected`, `NeedsAuthentication`, `PendingApproval`, `Disabled`, `Failed { detail }`, `Unknown` — with `Display`/`FromStr` and no `_ => default` arm anywhere it is matched; verify a round-trip test covers every variant and that adding one breaks the match rather than falling through
- [x] 1.3 Define `ServerRow { name, target, state }` with `Target::{Http { url }, Stdio { command } }` and a `short_label()` that yields host for HTTP and the program's basename for stdio; verify a 1.5 KB `node -e` command yields a label under the cap and that `short_label()` never returns the full command
- [x] 1.4 Define `Inventory` as the probe's outcome — rows, the instant the probe ran, and a distinct `Unprobeable` for a type with no list command — so "none configured" and "cannot tell" are different values, not an empty vec twice; verify the spec's two scenarios map to two distinct variants

## 2. `knot-mcp-probe`: running and parsing

- [x] 2.1 Implement the runner: spawn the type's list command with the agent's working directory and launch environment, stdin closed, stdout and stderr captured, killed at `PROBE_TIMEOUT` from `consts.rs`; verify a fixture command that sleeps past the timeout is killed and yields `ProbeError::TimedOut` rather than hanging the test
- [x] 2.2 Implement the Claude Code parser: name is the text before the first `": "`, state is anchored on the last ` - ` followed by a recognized state glyph, and the target is what lies between; verify against a captured fixture holding the `node -e` stdio entry, an `(HTTP)` entry, `⊘ Disabled for this project`, `✘ Failed to connect — -32602: Invalid request parameters`, and a name containing both spaces and colons
- [x] 2.3 Map every observed state string to `ServerState`, retaining the failure detail; verify `✘ Failed to connect — <error>` keeps `<error>` and that an unrecognized glyph yields `Unknown` rather than `Connected`
- [x] 2.4 Degrade per line, not wholesale: an unparsable line becomes an `Unknown` row; output with no parsable line at all is `ProbeError::Unrecognized` carrying its first line; verify a fixture of entirely foreign output produces the error and not an empty inventory
- [x] 2.5 Add `is_knot_endpoint(target, knot_url)` normalizing scheme, host, port and path before comparing; verify `http://127.0.0.1:8767/mcp` matches `http://localhost:8767/mcp/` and does not match a different port

## 3. Per-type knowledge on the roster

- [x] 3.1 Add an `mcp_list_command` column to `AgentTypeInfo`, populated for Claude Code and left empty for types whose output shape is not yet captured; verify the roster's existing completeness test covers the new column and that an empty value reads as "cannot determine" rather than "none"
- [x] 3.2 Add an `mcp_manage` column as an enum — a command naming one server, an interactive flow to drop into, or nothing — rather than a nullable string; verify Claude Code resolves to the interactive flow, OpenCode to the per-server command, and a shell agent to nothing
- [x] 3.3 Extend the roster test so a known non-shell type missing both columns fails rather than silently shipping an unprobeable type; verify the test fails when a column is removed from a populated row

## 4. Panel state and scheduling

- [x] 4.1 Add `crates/knot/src/workspace_window/mcp_panel/` with `mod.rs` declaring only, and siblings split by concern (`state`, `probe`, `render`, `actions`); verify `make size-check` and the declared-modules check both pass
- [x] 4.2 Model per-agent probe state as `NotProbed | InFlight | Done { at, outcome }` with a changed flag; verify a second request while `InFlight` is dropped rather than queued, and that the dropped request still sees the in-flight result
- [x] 4.3 Run the probe on `spawn_blocking`, never on the render path; verify no probe call is reachable from a `render` function, the way `diff_stats.rs` guards the same rule
- [x] 4.4 Add the single branch in `repaint_poll_tick` that takes the changed flag and notifies, and verify no other reader calls the clearing read — the breakage named four times in `.claude/rules/rust-structure.md`
- [x] 4.5 Gate probing on a running Panel-mode agent whose pane is shown; verify a hidden pane, a stopped agent and a Terminal-mode agent each probe zero times, and that switching a probing agent to Terminal mode abandons the in-flight result rather than reporting it

## 5. The list the section shows

- [x] 5.1 Compose the row list from Knot's own server (state read from `mcp_status.rs`, not the probe) followed by the agent's rows; verify Knot's row renders with its state before any probe has completed
- [x] 5.2 Merge a probed row whose target is Knot's endpoint into Knot's own row, keyed on endpoint rather than name, carrying Knot's state and a marker that the agent configures it independently; verify a differently-named entry on Knot's endpoint merges and a same-named entry on another endpoint does not
- [x] 5.3 Render Knot's row with no delegated action in every state including failed; verify the failed case offers none
- [x] 5.4 Render each row as name, transport and state using `short_label()`, with a copy action carrying the full target; verify the `node -e` fixture row's height matches a one-line row's

## 6. Header, states and refresh

- [x] 6.1 Implement the collapsed header: count and name the servers needing attention (needs-authentication or failed), deduplicated, capped, with a remainder counting those the names do not cover; verify four servers with two needing authentication name exactly those two
- [x] 6.2 Implement the remaining header branches — all connected, no servers configured, cannot determine, probe failed, probe in progress, agent not running — and verify each resolves from the catalog with no placeholder left behind
- [x] 6.3 Show when the rows were taken alongside them, and never present them as live session state; verify the timestamp is present whenever rows are
- [x] 6.4 Keep the previous probe's rows when a later probe fails, shown with their original timestamp beside the failure; verify a failure after a success does not empty the list
- [x] 6.5 Add the refresh action and the expanded header's row count; verify refresh while in flight starts no second probe

## 7. The delegated action

- [x] 7.1 Offer the action only on an agent row in the needs-authentication, failed, pending-approval or disabled state, and only for a type with an `mcp_manage` value; verify a connected row, an unknown row and a row on an unprobeable type each offer none
- [x] 7.2 Spawn a plain terminal in the agent's working directory through the path `acp-panel-ui`'s "Terminal remains available" already provides, and inject through `send_text`/`send_return`; verify the agent's ACP session is untouched and the adapter subprocess is never the injection target
- [x] 7.3 Build the injected command from the roster only. For a per-server command, shell-quote the server name and reject it against a conservative pattern, falling back to the interactive flow when it fails; verify a name carrying `;`, `$(` or a newline never reaches the command line
- [x] 7.4 Re-probe when the delegated terminal exits; verify the section shows the new result and that a terminal closed without action leaves the rows consistent rather than cleared

## 8. Localization

- [x] 8.1 Add catalog keys for every `ServerState`, every header branch, the timestamp phrasing, the independent-registration marker and the action labels; verify tests assert the keys resolve and never the English copy
- [x] 8.2 Touch `knot-core` after editing `en.yml` so the l10n tests do not run against a stale artifact

## 9. Gate

- [x] 9.1 Run `make` end to end — `fmt-check`, `size-check`, `lint`, `test`, `build`; verify no file crossed 700 lines and no crate-wide `allow` was added
- [x] 9.2 Verify by hand in a debug build against a real Claude Code agent: a server disabled for the project reads as disabled rather than failed, the delegated action lands in `/mcp`, and the section catches up when that terminal exits
- [x] 9.3 Verify the cost claim by hand: a window showing the dashboard, a Terminal-mode agent or a stopped agent runs zero probes, and a shown Panel agent runs one probe rather than one per tick. (Corrected from "never expanding the section runs zero probes": the sampler's gate is *shown*, not *expanded*, because the collapsed header names what needs attention and so needs an answer too — `openspec/specs/agent-mcp-status/spec.md`, "Probing is bounded and runs off the render path".)
