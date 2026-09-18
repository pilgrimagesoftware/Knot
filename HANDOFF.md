# Hand-off: acp-only-agent-launch

Branch: `acp-only-agent-launch`, 40 commits ahead of `develop`. `make rust`
passes clean. Delete this file once the open items below are closed.

The branch started as the OpenSpec change `acp-only-agent-launch` (ACP-only
launch for non-shell agents, plus the MCP wiring fix). Everything after that
came out of live testing in the running app, which surfaced a long tail of
panel, dialog and window defects - many of them pre-existing bugs that the
change simply made load-bearing.

## Verified working in the app

- Knot's own MCP server reaches ACP-launched agents; `mcp__knot__*` tools
  register and `set-status` works.
- Claude and OpenCode connect, stream, and render tool calls.
- The colored diff stat in the workspace header.
- Panel input controls and the Send button (they were rendering below the
  window edge).

## Root causes worth knowing

Four bugs were the same species: **the code modelled a protocol from
plausible inference rather than from the spec, and failed silently.** If a
feature "does nothing", suspect this shape first.

- `tools/list` serialized `input_schema`; MCP requires `inputSchema`. A
  validating client drops every tool in the list, so Knot's whole tool set
  was invisible. It looked like a port collision because a *different* MCP
  server in the user's own config answered to the same tool names.
- ACP has no `tool_call_result` or `diff` session update. A call's output
  arrives as `content` on `tool_call`/`tool_call_update`, so every card read
  "Running…" forever.
- ACP has no `turn_end` update either - a turn ends by *responding* to
  `session/prompt`. The response was discarded, so `turn_active` stayed true
  from the first prompt onward and the Send button and Enter key were dead.
- `session/request_permission` nests `params.toolCall.toolCallId`; we read a
  flat `params.toolCallId`. The existing test asserted the wrong shape, so it
  passed against the bug.

Two more worth remembering:

- **Dialogs never rendered.** `gpui_component::Root::render` does not draw
  `active_dialogs`; the application's own root view must render
  `Root::render_dialog_layer`. No window did, so *every* confirmation in the
  app opened invisibly. Upstream documents the exact symptom on that
  function. Found by instrumenting the click path after two wrong guesses -
  the handler fired and the alert's build closure never did.
- **`min_w_0()`** accounts for four separate layout bugs here. GPUI inherits
  CSS's `min-width: auto`: `flex_1()` says "may grow", only `min_w_0()` says
  "may shrink". Any flex row holding user text needs it.

## Open items

1. **Ctrl-C handling is unverified.** `quit_on_terminal_signals` builds and
   is wired at startup but was never confirmed against a running app.
2. **Gemini connects only intermittently.** Measured from a bare shell with
   no Knot involved, `gemini --acp --skip-trust` failed to answer
   `initialize` 3 runs in 5. Knot's behaviour is correct (it times out at
   20s and now offers "Try again"). The machine was heavily loaded during
   the measurement, so load and the `aws-mcp` entry in
   `~/.gemini/settings.json` (gemini prints "MCP issues detected" every run)
   are both untested explanations. Re-measure on an idle machine.
3. **The agent context menu is missing eight items** the Swift reference has
   (`Skwad/Views/Components/AgentContextMenu.swift`): New Companion, Fork
   Agent, Duplicate Agent, Move to Workspace, Save to Bench, Open In,
   Markdown Files, Register Agent - plus its dividers and the
   `AgentMenuVisibility` rules. Much of the backing exists already
   (`move_to_workspace` in the store, `BenchAgent`, `fork_session`, the MCP
   register tool), so it is mostly UI wiring. Worth an OpenSpec change
   rather than a patch.
4. **Companion-exit closing the agent is new behaviour**, not in
   `agent-lifecycle`. Write it into the contract or reconsider it.
5. **Restart deliberately drops the session**, per `agent-lifecycle`'s
   Restart requirement, with Resume as the separate operation. Reviewed and
   left as-is; changing it is a spec change.
6. `openspec/changes/acp-only-agent-launch/tasks.md` still has 5.2, 5.3 and
   6.6 unchecked - all manual verification.

## Notes for whoever picks this up

- Agents running *inside* Knot were pointed at this same checkout during
  testing, and their edits landed in the working tree. One such change (the
  panel-slot deadlock fix, commit "publish the panel slot before the
  registration turn") is genuinely good and is committed with a trailer
  saying where it came from. Don't run Knot agents against this repo while
  editing it.
- `cargo +nightly fmt` resolved to the stable binary until `rustup toolchain
  install nightly --component rustfmt` was run. The nightly binary also
  works directly:
  `~/.rustup/toolchains/nightly-aarch64-apple-darwin/bin/rustfmt --edition 2024 <files>`
- The PTY tests' timeouts were widened to a named 60s failsafe after they
  failed under compile load and passed on rerun. Measured: the suite takes
  ~10s with the machine saturated, so the old 5s budget sat inside the noise.
  `knot-discovery`'s watch tests were measured too (12/12 green) - the
  flakiness the previous hand-off recorded for them is gone.
