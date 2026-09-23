# Proposal

## Why

An agent left running in Knot spawns processes the user never sees: a dev server started
with a background bash call, a watcher, a test runner that outlived its tool call. Nothing
in the UI reports they exist, so the only evidence is a busy machine or a port that is
already bound, and the only remedy is `ps` in another terminal. Knot already owns the root
process of every agent session — the PTY shell for shell agents, the ACP adapter subprocess
for panel agents — so it is the one place that can attribute a stray process to the agent
that started it.

## What Changes

- Add a process-tree reader that, given a root PID, enumerates the live descendants of an
  agent's session: PID, parent PID, command line, start time, elapsed runtime.
- Classify each descendant as **background** or **foreground**: on a PTY-backed agent, a
  descendant outside the terminal's foreground process group is background. An ACP agent has
  no controlling terminal, so every descendant of its adapter is background.
- Surface the result as a collapsible **Processes** section inside the selected agent's pane,
  for both terminal (PTY) and panel (ACP) agents. Collapsed, it shows a count; expanded, one
  row per process with its command, runtime, and PID.
- Per-row actions: **terminate** (SIGTERM, escalating to SIGKILL after a grace period, behind
  a confirmation), **copy PID**, **copy full command**, and **open Activity Monitor**.
- Sample the tree off the render path, on a timer, only while a section is expanded, and
  publish snapshots into window state the renderer reads — no process enumeration during a
  GPUI frame.
- Add `sysinfo` to the workspace dependencies as the cross-platform process source.
  (Superseded during design: `design.md` reads the table with `ps` instead, because
  `Cargo.lock` already carries `sysinfo` transitively and a second major version of it
  would be compiled for six fields. No workspace dependency was added.)

Non-goals, stated so the boundary is explicit:

- No change to session teardown. Deactivating, restarting, removing an agent, or quitting the
  app continues to do exactly what it does today; this change does not start reaping orphans,
  and the quit warning does not learn about leftover processes.
- No persistence. Snapshots are runtime-only; nothing about a process is written to settings.
- No history. The section shows what is alive now, not what an agent ran and exited.
- No global view. Processes are shown per agent, not aggregated into the dashboard or a
  workspace-wide takeover view.
- No CPU or memory columns. Identification and control, not monitoring.

## Capabilities

### New Capabilities
- `agent-processes`: enumerating the live descendant processes of an agent's session root,
  classifying them as background or foreground, presenting them in the agent's pane, and
  terminating or identifying an individual process.

### Modified Capabilities

None. The section is additive: it reads process state that already exists and adds no
requirement to how agents are created, launched, torn down, or persisted.

## Impact

- **New crate `knot-processes`**: descendant enumeration over `ps` (see above), foreground
  process-group lookup, and termination with escalation. Runtime-agnostic, no async runtime;
  callers wrap blocking sampling in `spawn_blocking`.
- **`knot-terminal`**: expose the PTY child's PID and the master fd's foreground process group
  so `PtyTransport`-backed agents can be sampled.
- **`knot-acp`**: expose the adapter subprocess's PID from `Transport`.
- **`crates/knot`**: a new pane section rendered in both the terminal view and the ACP panel
  pane; per-agent expansion state and the sampling task that feeds it; the terminate
  confirmation; clipboard and Activity Monitor actions alongside the existing `open_in`
  helpers.
- **`knot-core`**: new `l10n` keys for the section header, the count, the row actions, and the
  terminate confirmation.
- **Workspace `Cargo.toml`**: unchanged - see the superseded `sysinfo` note above.
- **Platform**: the foreground/background split and termination are Unix paths. CI builds on
  Linux as well as macOS, so both must compile; behavior is specified for Unix and the
  classification degrades to "all descendants are background" where no controlling terminal
  is known.
