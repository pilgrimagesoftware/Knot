# Design

## Context

See `proposal.md` — Why. The constraints that shape the approach:

- **The submit path is one chokepoint.** `send_panel_prompt`
  (`crates/knot/src/workspace_window/panel/prompt.rs:300`) reads the textarea,
  trims, appends pending context lines, and calls `deliver_panel_prompt`.
  Nothing between the composer and the agent inspects the text today —
  `panel-slash-commands` is deliberately a completion popup, and its spec says
  so: "nothing is executed by the panel". This change introduces the first
  client-side interception, so it has to be visibly different from `/`, not a
  quiet extension of it.
- **Process running already has a house style.** `knot-processes::command`
  (std-only crate, `thiserror` its single dependency) runs a child with piped
  stdio, drains both streams on worker threads so a full pipe never wedges the
  child, polls for exit, and kills on a wall-clock timeout. `knot-git`'s runner
  is the same shape with a cwd. Neither streams output, takes a cancel signal,
  or bounds capture size.
- **Off-thread results have to reach a frame.** GPUI re-renders per keystroke
  and does no I/O on the render path. `PanelSessionHandle` already models the
  pattern: shared state behind a lock, an `AtomicBool` dirty flag, and a
  clearing read in `repaint_poll_tick`'s `if` chain. `.claude/rules/
  rust-structure.md` names four defects from breaking that chain.
- **ACP carries one text block.** `knot-acp`'s `session/prompt` sends a single
  text block, which is why attached files already ride along as
  `"\n\nAttached: <path>"` lines (`prompt.rs:311-319`). Shell results reach the
  agent the same way; no wire shape is invented.
- **`rich-prompt-composer` is in flight** and owns the composer's token
  scanner and styling. This change adds a third trigger character to a
  scanner that is being rewritten concurrently.

## Goals / Non-Goals

**Goals:**

- One interception point, not a dispatch table. `!` is the only trigger this
  change adds, and the decision to divert is made in one place.
- A shell command's lifetime is fully independent of the ACP session's:
  neither can cancel, block, or reorder the other.
- Command execution reuses `knot-processes`' existing discipline (worker-thread
  drain, wall-clock kill) rather than introducing a fourth process-spawning
  pattern into the workspace.
- The change lands whether or not `rich-prompt-composer` has merged.

**Non-Goals:**

- A persistent shell session. Each command is a fresh process; see the
  "Commands run non-interactively" requirement.
- Making `knot-processes` async. It stays std-only and runtime-agnostic.
- Reworking how attached context reaches the agent. Shell results use the
  existing line-append convention and inherit its limitations.

## Decisions

### Recognise `!` on the raw buffer, before trimming

`send_panel_prompt` trims first. The trigger test runs on the untrimmed value:
`!` must be byte 0. Everything else — including the trim — is unchanged.

This makes a single leading space the escape, which costs nothing to implement
and nothing to explain, and it never mutates what the user typed.

*Alternatives:* `\!` or `!!` as an escape — both rewrite the buffer before
sending, so what reaches the agent is not what the user sees. Rejected;
`rich-prompt-composer` makes the same commitment for its own tokens.

*Rejected:* recognising `!` anywhere at the start of a line (the rule `/` uses
via `active_token`). A `/` token is completed in place mid-buffer; a `!`
consumes the whole buffer, so line-start recognition would make a multi-line
prompt containing an exclamation-led line silently executable.

### Trigger recognition lives in `panel_commands/token.rs`

`!` joins `/` (and `@`, once `rich-prompt-composer` lands) in the module that
already answers "what kind of token is the composer holding". The composer's
indication is then one more consumer of that answer.

Decoupling from `rich-prompt-composer`: the indication this change ships is a
label in the control row driven by the recognition result, not a text
decoration. If the rich composer merges first, a `Shell` construct in
`composer_scan` styles the line as well; if it merges second, it reads the
same recogniser. Neither change blocks the other.

### A new `knot-processes::shell` module, not an extension of `command::run`

`command::run` is blocking-to-completion, has no cwd, no cancel handle, and no
size bound. Bending it to cover both uses would leave one function with two
incompatible lifetimes. The new module keeps the crate's existing mechanics —
`std::process::Command`, `Stdio::piped()`, worker threads draining each stream,
a poll loop that kills on timeout — and adds:

- `cwd`, set from the agent's `Agent::folder`.
- A `ShellRun` handle: `Arc<Mutex<ShellRunState>>` holding the two output
  buffers and the run's status, plus an `Arc<AtomicBool>` dirty flag the
  worker threads set on every append.
- `cancel()`, which kills the process group.

The crate stays std-only and runtime-agnostic, consistent with `knot-git`.

*Alternative:* `tokio::process` on the panel's existing runtime. Rejected — it
would put a tokio dependency in a crate that deliberately has none, to gain
nothing: the work is two blocking pipe reads, which is what the existing worker
threads already do well.

### Kill the process group, not the child

The child is spawned in its own process group
(`std::os::unix::process::CommandExt::process_group`), and both cancel and
timeout signal the group. `! npm test` spawns a tree; killing only `sh` orphans
it, and the orphan keeps holding the pipe the drain threads are reading.

### Capture: two buffers, head-truncated, with separate limits from the agent hand-off

stdout and stderr are captured separately — there is no PTY, so merging them
into one stream invents an interleaving that never existed and loses which
stream carried a failure. The entry renders them as one block with stderr
distinguished.

Capture stops at a byte limit (`SHELL_OUTPUT_LIMIT`) and keeps the head; the
command continues, its entry marked truncated. The head is where a command
states what it could not do.

*Alternative:* head-and-tail with an elision marker, which serves `!cargo test`
better (the verdict is at the end). Rejected for now at this size limit; it
doubles the buffer bookkeeping for a case a 256 KiB head rarely reaches. If it
proves wrong, it is a change inside the capture module with no spec impact.

Constants (`SHELL_OUTPUT_LIMIT`, `SHELL_TIMEOUT`) live in
`knot-processes::consts` beside `DEFAULT_TIMEOUT`. `SHELL_TIMEOUT` is far
longer than `DEFAULT_TIMEOUT`'s 10 s — a test run is a legitimate `!` command —
so it is its own constant rather than a reuse.

### `$SHELL -lc`, falling back to `/bin/sh -c`

A login shell is what makes `! npm test` find `npm`. The application is
launched from Finder with a GUI environment whose `PATH` does not include what
the user's profile adds, and a user typing `!` expects their terminal's
environment, not the app's.

*Trade-off:* login shells run profile files, so a profile that prints a banner
prints it into every command's output, and startup costs tens of milliseconds
per command. Both are visible rather than hidden, which is the right failure
mode; a user who dislikes it can quiet their profile.

*Alternative:* `-c` alone, matching how the ACP adapter subprocess is spawned.
Rejected — the adapter's environment is the app's by design; the user's `!`
command's is not.

### A new `PanelMessage::Shell` variant holding plain data, with live runs in a side table

`PanelMessage` derives `Clone`/`PartialEq` and is compared per frame, so the
variant carries a plain `ShellCard { id, command, cwd, stdout, stderr, status }`
— not a lock. Live `ShellRun` handles live in a side table on the panel
(`panel_shell_runs: HashMap<ShellRunId, ShellRun>`), and `repaint_poll_tick`
copies from any dirty handle into its card and clears the flag. A finished run
is removed from the table; its card stays in the conversation.

This is the `PanelSessionHandle` pattern applied to a second kind of
off-thread work, and it keeps the clearing read on the one path that renders —
the failure mode `.claude/rules/rust-structure.md` warns about.

`status` is an enum (`Running`, `Exited { code }`, `Signalled`, `Cancelled`,
`TimedOut`, `FailedToStart { message }`), per the workspace's closed-vocabulary
rule — no `String` with a `_ => default` arm.

### Pending results are a sibling of `panel_pending_context`

`panel_pending_shell: HashMap<Uuid, Vec<ShellResultRef>>` holds, per panel, the
ids of finished-on-their-own runs not yet shared. `send_panel_prompt` drains it
alongside `panel_pending_context` and appends each as a block of the same shape
as the existing `"\n\nAttached: "` lines, then marks each card shared.

Only `Exited`/`Signalled` runs enter the pending list; `Cancelled`, `TimedOut`
and `FailedToStart` never do, and a discard control removes an entry from the
list without touching the card.

*Alternative:* sending the result to the agent immediately as its own turn.
Rejected by the user: it spends a turn per command and collides with an
in-flight response — the very independence this design is built around.

### The send control's enabled state is computed from the recognition result

The existing disable conditions (empty input, pending permission) already run
per frame in the input area. Shell recognition is a cheap prefix test on a
value the control row already reads, so it joins that computation rather than
becoming separate state that can drift from the buffer.

## Risks / Trade-offs

- **A `!` command runs with no confirmation, in the agent's worktree.** →
  Accepted: the user typed it, which is the same trust level as the embedded
  terminal view that already exists beside the panel. The trigger is reachable
  only from the composer; no agent, tool call, or MCP path can reach it. The
  spec says so explicitly, so a later change cannot widen it by accident.
- **Command output is sent to the agent, and a command like `!env` prints
  secrets.** → Results are pending, not sent, until the user's next prompt;
  the entry says so and offers a discard control. The window between running a
  command and sending a prompt is the user's chance to notice.
- **A login shell's profile banner lands in every command's output.** →
  Visible, self-explanatory, and fixable by the user. Preferred over a `PATH`
  that silently cannot find their tools.
- **Orphaned process trees from a cancelled or timed-out command.** → Process
  group kill. Residual risk: a process that ignores the signal keeps the pipe
  open; the drain threads exit on their own read error, and the entry is
  already marked terminal, so the panel is not held hostage.
- **`knot-processes` grows a streaming API with one caller.** → Accepted. The
  alternative is putting process management in the UI crate, where the render
  path is, which is worse.
- **`rich-prompt-composer` and this change both touch
  `panel_commands/token.rs` and the composer's control row.** → Whichever
  merges second rebases onto a small, well-bounded surface; neither depends on
  the other's modules existing.
- **`prompt.rs` grows toward the 700-line cap.** → Shell submission, the
  pending-result table, and the poll-tick drain go in a new sibling module
  under `panel/`, not into `prompt.rs`; `prompt.rs` gains only the branch.

## Migration Plan

Additive; no data migration and no persisted state. A buffer beginning with
`!` currently reaches the agent as text, so the only behavioural break is for a
user who deliberately opens a prompt with `!` — served by the leading-space
escape.

Rollback is removing the recognition branch in `send_panel_prompt`: the
capability's modules become unreachable and the panel's behaviour returns
exactly to today's.

## Open Questions

- Whether `SHELL_OUTPUT_LIMIT` and `SHELL_TIMEOUT` should become user settings
  rather than constants. Deferrable: constants first, and promoting one to a
  setting later changes neither the specs nor the module boundaries.
- Whether a finished command's entry should offer a re-run control. Not
  specified here; it is additive to the entry and changes nothing else.
