# Proposal

## Why

The same defect `gh-binary-discovery` fixed in `knot-forge`, one crate over.

`knot_git::Runner` spawns `Command::new("git")` and sets nothing on the
child. Under launchd's `/usr/bin:/bin:/usr/sbin:/sbin` that works for most
people, because `/usr/bin/git` is the Xcode command line tools shim. It does
not work for a user whose only git is Homebrew's at `/opt/homebrew/bin/git`
with no tools installed, and the git panel (#317) turns that from a
diff-stat card quietly reading zero into a foreground feature that fails.

Git's own helpers have the same problem regardless of where git itself is:
credential helpers, hooks, `git-lfs` and `ssh` are looked up in the
environment git is handed, which is this process's sparse one.

Raised from the git panel branch rather than found in the wild: that work
increases the call volume through `Runner` considerably, and asked whether
the resolution strategy should change before it does.

## What Changes

- `knot-git` accepts the git it should run - a program and an optional
  `PATH` for the child - through a set-once, process-wide
  `program::configure`. Unconfigured, it runs the bare `git` exactly as it
  does today.
- The `knot` binary resolves both at startup through
  `knot_core::exec_path` and configures the crate before anything builds a
  runner.
- Nothing at any call site changes. `Repository::open` is constructed from
  three crates and a dozen places, none of which has an opinion about which
  git; threading a value through all of them would be the same value
  arriving at the same place by a longer road, and would collide with the
  git panel branch's new call sites.

Non-goals:

- `knot-git` does not gain a `knot-core` dependency. The crate is standalone
  by design (the workspace layout in `AGENTS.md`), and searching the
  standard install locations is `knot-core`'s job. It accepts an answer; it
  does not go looking for one.
- No change to which git is chosen on a working machine. The merged path
  puts the process's own entries first, so `/usr/bin/git` still wins wherever
  it exists. This only decides the case where today there is no answer.
- No setting for the git path, and no version check. A machine with two gits
  gets the one its `PATH` ordering already prefers.

## Capabilities

### Modified Capabilities

- `git-operations`: the command runner requirement gains the binary it runs -
  today the spec says only that git is invoked, leaving "which git" to the
  reader.

## Impact

- `crates/knot-git/src/program.rs` (new), `runner.rs`, `consts.rs`, `lib.rs`.
- `crates/knot/src/external_tools.rs` (new), `main.rs` - one call, placed in
  `main` rather than `app_bootstrap` both because it must precede every
  runner and to stay clear of the appearance-mode work in that file (#281).
- No dependency change in either crate: `knot-git` uses `std::sync::OnceLock`,
  and the binary already depends on `knot-core` and `knot-git`.
