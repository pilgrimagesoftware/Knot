# Tasks

## 1. The `knot-processes` crate

- [x] 1.1 Create `crates/knot-processes` with `lib.rs` (module doc linking
      `openspec/specs/agent-processes/spec.md`), `consts.rs`, and a `thiserror` `ProcessError`
      plus a crate `Result` alias; add it to the workspace members and verify `cargo build
      --workspace` succeeds.
- [x] 1.2 Implement the process-record parser for `ps -Ao pid=,ppid=,pgid=,tpgid=,etime=,command=`
      output — five leading fields then the command verbatim — and verify unit tests over
      captured fixture text cover a command containing spaces, leading whitespace padding, a
      malformed line being skipped without failing the sample, and both `etime` shapes
      (`MM:SS`, `HH:MM:SS`, `DD-HH:MM:SS`).
- [x] 1.3 Implement the descendant tree walk: given a root PID and a parsed table, return every
      transitive descendant excluding the root; verify unit tests cover a grandchild being
      reported, the root being excluded, an unrelated process with a recycled-looking PID being
      excluded, and a cycle in the parent chain terminating rather than looping.
- [x] 1.4 Implement background/foreground classification (`pgid == tpgid` is foreground,
      everything else including `tpgid` of `0`/`-1` is background) and verify unit tests cover a
      foreground command, a detached process, and a subtree with no controlling terminal.
- [x] 1.5 Implement the blocking sample entry point that runs `ps`, parses, and returns the
      table; verify an integration test samples the current process and finds this test binary's
      own PID in the table.
- [x] 1.6 Implement termination: re-verify the target's parent chain and start time, send `TERM`,
      poll for exit to a bounded grace period, then send `KILL`; verify tests cover a spawned
      sleep exiting on `TERM`, a `TERM`-ignoring child being killed after the grace period, an
      already-exited PID reported as success, and a mismatched identity refused without
      signalling.
- [x] 1.7 Verify `make lint` and `make size-check` pass for the new crate and that `mod.rs`
      files declare and re-export only.

## 2. Root PID accessors

- [x] 2.1 Expose the PTY child's PID from `knot-terminal` (`PtyTransport` through its session
      type) as `Option<u32>`; verify a test spawns a shell session and observes a PID that is
      present while running and absent after the child exits.
- [x] 2.2 Expose the adapter subprocess's PID from `knot-acp`'s `Transport` as `Option<u32>`;
      verify a test spawns a stub adapter and observes the PID.
- [x] 2.3 Add the session-root lookup in `crates/knot` that resolves an agent to its root PID —
      PTY child for shell agents, adapter for panel agents, `None` when not running — and verify
      unit tests cover a shell agent, a panel agent, a deactivated agent, and an agent mid-restart.

## 3. Sampling task and window state

- [x] 3.1 Add per-agent processes-section state (collapsed by default, expansion, last snapshot,
      last failure, in-flight terminations) to the workspace window; verify unit tests cover
      default-collapsed and toggle behavior.
- [x] 3.2 Implement the per-window sampling task: one `ps` read per interval on a blocking task,
      descendant sets computed per observed root from that one snapshot, published into window
      state; verify a test with two observed roots performs a single sample and populates both.
- [x] 3.3 Gate the task's lifetime — starts when a section expands, stops on collapse, on the
      agent stopping, on the pane no longer being shown, and on window close; verify unit tests
      cover each stop trigger.
- [x] 3.4 Verify by inspection and test that no render path calls into `knot-processes`: the
      renderer reads only the published snapshot, and a render with no completed sample yields
      the unknown count rather than zero.

## 4. Pane section UI

- [x] 4.1 Add the localization keys for the section label, count, column and classification
      labels, row actions, terminate confirmation, empty states, and failure notices to
      `crates/knot-core/locales/en.yml`; verify the l10n key tests resolve each key (touch
      `knot-core` first so the catalog is not read from a stale build artifact).
- [x] 4.2 Build the collapsed section header showing the label and the background count (unknown
      before the first sample); verify a test asserts the three states — unknown, zero, positive.
- [x] 4.3 Build the expanded row list: background rows before foreground, longest-running first,
      each row showing single-line truncating command, runtime, PID, and classification; verify
      tests cover the ordering and that a long command renders one line with the trailing fields
      still present.
- [x] 4.4 Render the empty and failure states — not running, nothing spawned, sample failed while
      keeping the last successful list, and silent recovery; verify a test drives all four.
- [x] 4.5 Mount the section in both the terminal view and the ACP panel pane; verify a test
      asserts the section is present in each.

## 5. Row actions

- [x] 5.1 Wire the terminate action: confirmation dialog naming the command, cancel changing
      nothing, confirm running the `knot-processes` termination sequence on a blocking task, the
      row marked terminating until the next sample; verify tests cover confirm, cancel, and
      already-exited.
- [x] 5.2 Wire copy-PID and copy-command to the system clipboard, copying the untruncated command
      line; verify a test asserts the copied command has no ellipsis.
- [x] 5.3 Wire the process-viewer action through `open_in` to open Activity Monitor on macOS, and
      omit the action where no process viewer is known; verify a test asserts the action's
      presence on macOS and its absence otherwise.
- [x] 5.4 Surface a refused signal as a localized failure with the row left in place; verify a
      test asserts the failure text resolves by key and the row survives.

## 6. Verification and close-out

- [x] 6.1 Run the full gate — `make` — and verify `fmt-check`, `size-check`, `lint`, `test` and
      `build` all pass on the workspace.
- [ ] 6.2 Manually verify against a live agent: start a shell agent, launch a background server
      from it, confirm it appears as background with a plausible runtime, confirm a foreground
      command is classified foreground, terminate the server from the row and confirm it
      disappears from the next sample.
- [ ] 6.3 Manually verify against a live panel agent: confirm the adapter's descendants appear,
      all classified background, and that collapsing the section stops sampling.
- [ ] 6.4 Verify the documented limitation holds and is not mistaken for a defect: a
      double-forked daemon started by an agent does not appear in the section, because it has
      been reparented away from the agent.
