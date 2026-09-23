# Design

## Context

See `proposal.md` — Why. The constraints that shape the approach:

- Knot spawns exactly two kinds of session process. `knot-terminal`'s `PtyTransport` owns a
  `portable_pty` `Child` (the interactive shell); `knot-acp`'s `Transport` owns a
  `tokio::process::Child` (the adapter). Neither exposes its PID today.
- `crates/knot` must not perform I/O on the render path — GPUI re-renders per keystroke, and
  the project has already paid for that mistake three times (`knot/src/diff_stats.rs`).
- `knot-git` and `knot-forge` establish the house pattern for reading OS state: a standalone,
  runtime-agnostic crate that runs a well-known command line, parses its output into typed
  values, and leaves async scheduling to the caller.
- CI builds the workspace on `ubuntu-latest` as well as `macos-latest`, so everything here must
  compile on Linux even though the product is macOS.

## Goals / Non-Goals

**Goals:**

- One sampling mechanism that serves both agent kinds, so the pane section does not branch on
  terminal-versus-panel beyond where it gets the root PID.
- A sample that costs one process-table read per window per interval, not one per agent.
- Termination that cannot hit the wrong process when PIDs are recycled.

**Non-Goals:**

- Beyond `proposal.md`'s non-goals: no attempt to recover processes that have been reparented
  away from the agent (see Risks), and no per-process resource sampling, which would force a
  higher-cost read on every interval.

## Decisions

### Read the process table with `ps`, not a process-info crate

`ps -Ao pid=,ppid=,pgid=,tpgid=,etime=,command=` returns the whole table in one line-oriented
read, with identical keyword support on macOS `ps` and Linux `procps-ng`. `knot-processes`
parses it into records and builds the descendant tree itself.

*Alternative — `sysinfo`:* the obvious crate, but `Cargo.lock` already carries `sysinfo`
0.31.4 pulled in transitively by a `gpui` dependency (`zed-scap`). Adding a current `sysinfo`
at the workspace level compiles a second major version of it, plus `rayon`, `ntapi` and
`windows`, for a feature that needs six fields. Pinning to 0.31 to dedupe means depending on
whatever that transitive edge happens to be, which is not ours to pin.

*Alternative — `/proc` on Linux and `sysctl(KERN_PROC_ALL)` on macOS:* no dependency, but two
platform implementations and `unsafe` FFI for a value `ps` already formats. The workspace's
rule is no panics and typed errors in library code; a hand-rolled `sysctl` path is the wrong
place to spend that budget.

The cost is parsing: `etime` is `[[DD-]HH:]MM:SS`, and the command column contains spaces, so
the parser splits exactly five leading whitespace-delimited fields and takes the remainder
verbatim. `ps` never emits an embedded newline in that column, so one line is one process.

### Classify foreground by comparing `pgid` to `tpgid`

A process is in its terminal's foreground process group exactly when its own process group id
equals the foreground process group id of its controlling terminal — both of which `ps`
already reports per process. Everything else is background.

*Alternative — `tcgetpgrp` on the pty master fd:* the textbook call, but it means exposing
`PtyTransport`'s master file descriptor out of `knot-terminal`, threading it to the sampler,
and guarding its lifetime against the transport being dropped mid-sample. The `ps` columns give
the same answer with no new cross-crate lifetime to get wrong.

A `tpgid` of `0` or `-1` — no controlling terminal, which is the ACP adapter's whole subtree —
falls through to background, which is what the spec requires anyway.

### A new standalone crate, `knot-processes`

Mirrors `knot-git`: no async runtime, `thiserror` error enum with a crate `Result` alias, and
a pure parse function that unit-tests against captured `ps` output with no live processes
involved. Modules by concern — the `ps` invocation, the record parser, the tree walk, the
signal path — with `mod.rs` declaring and re-exporting only, per the workspace rules.

Callers wrap the blocking sample in `spawn_blocking`.

### Sample once per window, not once per agent

`ps -A` already returns every process, so the expensive part is the same whether one agent is
observed or six. One sampling task per workspace window reads the table on an interval, then
computes a descendant set per observed root PID from that single snapshot. Adding a second
expanded section costs a tree walk over data already in memory.

The task runs only while at least one section in that window is expanded and at least one
observed agent is running, per the spec; it publishes a snapshot into window state and the
render reads it. That is what keeps process enumeration off the render path.

### Source the root PID from the two transports

`knot-terminal` gains a PID accessor on `PtyTransport` (from `portable_pty`'s
`Child::process_id`) surfaced through its session type; `knot-acp` gains one on `Transport`
(from `tokio::process::Child::id`). Both return `Option<u32>` — absent once the child has been
reaped — which maps directly onto the spec's "a stopped agent has no session root".

### Terminate through `kill`, re-verifying identity first

Same reasoning as reading: a `kill` subprocess avoids a `libc` dependency for one call. The
sequence is: re-read the process table, confirm the target PID still has the parent chain and
start time the row was built from, send `TERM`, poll for exit until the grace period elapses,
then send `KILL`. The re-verification is what makes PID reuse harmless.

The whole sequence runs on a blocking task; the UI shows the row as terminating until the next
sample, rather than removing it optimistically.

### Activity Monitor via the existing open-in path

`crates/knot/src/open_in.rs` already launches macOS applications for an agent's folder. The
process-viewer action reuses it to open Activity Monitor and nothing more. macOS exposes no
supported way to select a specific process row in Activity Monitor from outside the app —
hence the spec pairs the action with copy-identifier rather than claiming a selection it cannot
perform.

## Risks / Trade-offs

- **A double-forked daemon is unattributable** → A process that forks twice and exits its
  parent is reparented to `launchd`/`init`, and no ancestry links it to the agent any more. It
  will not appear in the section. Nothing short of process-group or session tracking at spawn
  time fixes it, which would mean owning how every agent launches its tools. Documented as a
  limitation rather than papered over: the section reports what descends from the agent, and
  says so.
- **PID reuse between sample and terminate** → Mitigated by re-verifying parent chain and start
  time immediately before signalling. A reused PID fails the check and the action reports
  nothing to terminate.
- **`SIGKILL` can lose a process's unsaved work** → Gated behind the confirmation dialog and
  reached only after the grace period; `TERM` is always tried first.
- **A signal can be refused** → A process not owned by the user running Knot rejects the
  signal; surfaced as a localized failure with the row left in place, never as a silent no-op.
- **`ps` output drift between platforms** → The six keywords used are POSIX-stable and
  supported identically by macOS `ps` and `procps-ng`. The parser tolerates extra leading
  whitespace and rejects a malformed line rather than the whole sample, so one odd row cannot
  blank the section.
- **Sampling cost while expanded** → One `ps -A` per window per interval, on a blocking task.
  Bounded by leaving the interval in `knot-processes`'s `consts.rs` and by sampling only while
  a section is expanded.

## Migration Plan

Additive. No persisted data changes shape, no existing requirement changes, and no agent
behavior changes; an installation that never expands the section does strictly what it does
today. Rollback is reverting the change — there is no migrated state to undo.
