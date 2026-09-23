# Proposal

## Why

Every CLI agent harness the user already works in — Claude Code, Codex, the
ACP adapters Knot drives — lets a `!` prefix drop straight to a shell: `! ls
-la` runs locally, the output lands in the transcript, and the agent picks it
up as context. Knot's panel has no such escape. A Panel-mode agent's user has
to leave the panel for the terminal view, run the command there, then copy the
output back into the prompt by hand — for the most ordinary question a person
asks mid-conversation ("what's actually in this directory?", "did the build
pass?"). The panel already knows the agent's worktree, already renders
tool-call output, and already has a token scanner for `/` and `@`; the missing
piece is the one trigger that runs something.

## What Changes

- A prompt whose first non-whitespace character is `!` is recognised as a
  shell command rather than a message to the agent. Submitting it runs the
  remainder of the line in a shell, in the selected agent's worktree, and the
  prompt is not sent to the agent.
- The command and its output render in the conversation as their own entry
  kind — command line, stdout/stderr, and exit status — distinct from a user
  message, an assistant message and a tool call.
- Execution is local and independent of the ACP session: a `!` command runs
  immediately whether or not a turn is in progress, and does not join the
  prompt queue, interrupt a turn, or wait for one.
- The result is held as pending context and attached to the user's next
  ordinary prompt, so the agent sees what the user ran and can react to it
  without a turn being spent on the command itself. Pending results that are
  never followed by a prompt are never sent.
- Running commands are interruptible and bounded: a long-running command can
  be cancelled from its entry, and output capture is capped so a runaway
  command cannot exhaust memory or wedge the panel.
- The composer marks a `!` line as a shell command while it is being typed, so
  the user can see before pressing Enter that the line will not reach the
  agent.

### Non-goals

- Interactive commands. The shell runs non-interactively with no TTY; a
  command that expects input (`vim`, `less`, a password prompt) is not
  supported. The embedded terminal view remains the surface for that.
- A `!` trigger in the terminal composer (`terminal-input`). That surface
  already delivers keystrokes to a real shell; intercepting `!` there would
  shadow a legitimate character.
- A lookup popup for `!`. The `/` and `@` triggers complete from a registry;
  a shell command has no such registry. No history or completion popup is in
  scope.
- Letting the agent run `!` commands. The trigger is the user's alone; agents
  request command execution through their own tool calls and the existing
  permission prompts.
- Persisting shell results across an app restart, beyond whatever the
  conversation history already retains.

## Capabilities

### New Capabilities

- `panel-shell-passthrough`: the `!` trigger — how a shell command is
  recognised in the composer, where and how it runs, how its output and exit
  status are captured and bounded, how it is cancelled, how its result is
  rendered, and how that result reaches the agent as context on the next
  prompt.

### Modified Capabilities

- `acp-panel-ui`: the conversation gains a shell-result entry alongside user
  messages, assistant messages, tool calls and errors; and the send control's
  rules change — a `!` line submits while a response is in progress instead of
  being enqueued, and is not blocked by a pending permission request.

## Impact

- `crates/knot/src/workspace_window/panel/prompt.rs` — `send_panel_prompt`
  gains the branch that diverts a `!` line before `deliver_panel_prompt`; the
  pending-context hand-off attaches held shell results to the next prompt.
- `crates/knot/src/panel_commands/token.rs` — token recognition extends past
  `/` and `@` to `!`, feeding the same scanner the `rich-prompt-composer`
  change is building (`composer_scan`); the `!` treatment lands in
  `composer_style`.
- `crates/knot/src/panel_state/message.rs` — a new `PanelMessage` variant for
  the shell entry, with a card struct shaped like `ToolCallCard` (command,
  cwd, output, exit status, running/cancelled state).
- `crates/knot/src/panel_view/message.rs` and `rows.rs` — a render arm and row
  handling for the new variant; reuses the existing monospace/code treatment.
- `crates/knot-processes/src/command.rs` — the existing blocking runner
  (worker-thread drain, wall-clock timeout, kill-on-timeout) gains a cwd and a
  cancellation handle, or a sibling runner is added beside it. No new
  process-spawning approach is introduced.
- Agent worktree comes from `Agent::folder` (`crates/knot-agents`), the same
  lookup the panel already does for the skill registry.
- Off-thread completion must reach a frame through `repaint_poll_tick`'s
  dirty-flag chain, as `PanelSessionHandle` already does.
- `knot-core` localization: new keys for the shell entry's chrome, the
  cancelled and timed-out states, and the composer's shell-line marker.
- No new external dependencies.
