# Hand-off: acp-only-agent-launch

The branch itself is merged (PR #123). This file survives it because two
items are still open, and because the root causes below are worth keeping
until they have a better home. Delete it once both open items close.

## Root causes worth knowing

Four bugs on that branch were the same species: **the code modelled a
protocol from plausible inference rather than from the spec, and failed
silently.** If a feature "does nothing", suspect this shape first.

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

Three more worth remembering:

- **Dialogs never rendered.** `gpui_component::Root::render` does not draw
  `active_dialogs`; the application's own root view must render
  `Root::render_dialog_layer`. No window did, so *every* confirmation in the
  app opened invisibly. Upstream documents the exact symptom on that
  function.
- **A window cannot open a dialog on itself from a menu action.** The macOS
  menu dispatches through `App::dispatch_action`, which runs the handler
  inside `active_window.update(...)`; a second `window.update` on that same
  window is re-entrant and gpui refuses it with `"window not found"` - the
  same message it uses for a *closed* window, which is what made this look
  like a lifetime bug. Defer the open with `cx.defer`. (Found via "About
  Knot"; PR #124.)
- **`min_w_0()`** accounts for four separate layout bugs here. GPUI inherits
  CSS's `min-width: auto`: `flex_1()` says "may grow", only `min_w_0()` says
  "may shrink". Any flex row holding user text needs it.

## Closed since the branch merged

- **Ctrl-C handling works.** Measured 2026-09-18: the built binary exits
  with code 130 on `SIGINT`. `quit_on_terminal_signals` does what it claims.
- **Gemini's intermittency did not reproduce.** `gemini --acp --skip-trust`
  answered an ACP `initialize` 5 runs out of 5, at load average ~4-5 - i.e.
  under load comparable to the original 3-in-5 failure, with gemini 0.46.0.
  The `aws-mcp` entry in `~/.gemini/settings.json` is ruled out as the
  cause: gemini still prints "MCP issues detected" on every run while
  answering `initialize` normally. Reopen with fresh numbers if it recurs.
- **Companion-exit behaviour is now in the contract**, as the OpenSpec
  change `shell-exit-removes-the-agent`. Note the correction it carries:
  the implementation removes any *shell* agent whose process exits,
  companion or not, which is what the requirement now says.
- **Restart deliberately drops the session**, per `agent-lifecycle`'s
  Restart requirement, with Resume as the separate operation. Reviewed and
  left as-is; changing it would be a spec change, not a fix.

## Open

1. **The agent context menu is missing eight items** the Swift reference has
   (`Skwad/Views/Components/AgentContextMenu.swift`): New Companion, Fork
   Agent, Duplicate Agent, Move to Workspace, Save to Bench, Open In,
   Markdown Files, Register Agent - plus its dividers and the
   `AgentMenuVisibility` rules. Tracked as the OpenSpec change
   `agent-context-menu-parity`.
2. `openspec/changes/acp-only-agent-launch/tasks.md` still has 5.2, 5.3 and
   6.6 unchecked. All three need a human watching the running app; that file
   records why an agent session cannot close them.

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
