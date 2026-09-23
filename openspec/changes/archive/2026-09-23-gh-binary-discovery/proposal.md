# Proposal

## Why

The pull request view says `gh` is not installed on machines where it is.

`knot-forge` spawns the tool by bare name, so the lookup runs against the
process's own `PATH`. Knot is a GUI app: launched from Finder it inherits
launchd's `PATH`, which is `/usr/bin:/bin:/usr/sbin:/sbin` and nothing else.
`gh` is installed by Homebrew into `/opt/homebrew/bin` or `/usr/local/bin` -
neither of which is on that list - so the spawn fails with `NotFound`, which
becomes `ForgeError::Missing`, which the view renders as "not installed". Run
the same build from a terminal and the feature works, which is what makes the
report confusing: the machine, the binary and the credentials are all fine.

The same bug was already found and fixed once, for ACP adapters
(`agent-launch-command`'s merged search path). What is missing is that the fix
was local to `knot-agent-launch`, so the next subsystem to spawn a tool -
`knot-forge` - reintroduced it.

## What Changes

- The standard install locations move into `knot-core` as one shared roster
  (`exec_path`), used by every subsystem that spawns an external tool, so
  "installed" means the same thing everywhere.
- `knot-core` gains a program lookup over that merged path, resolving a bare
  name to an absolute path. Setting the child's `PATH` alone would suffice on
  today's `std` - it drops to `fork`/`exec` when the program is a bare name
  and `PATH` was set, so the supplied environment is the one searched - but
  that is std's implementation rather than its contract, and the C function
  underneath searches the calling process's `PATH`. Resolving first also makes
  "not installed" a decision about named directories rather than a reading of
  a spawn error.
- `GhRunner` spawns the located `gh`, and gives the child the merged `PATH`:
  `gh` shells out to `git` and to credential helpers itself.
- A machine with no `gh` anywhere on the merged path still reports Missing,
  unchanged.

Non-goals:

- No new install locations. The roster is the five `knot-agent-launch`
  already used; broadening it is a separate decision with its own evidence.
- No reading of the user's login shell environment (`$SHELL -lc 'echo $PATH'`).
  It would cover more cases, and it makes app startup depend on a subprocess
  that can hang on a slow shell profile - a larger change than this bug is.
- No setting for the `gh` path. Nothing suggests the standard locations are
  insufficient; a setting can be added when a machine is found that needs one.
- `knot-git` is untouched. `git` lives in `/usr/bin` on macOS, which the
  launchd `PATH` names.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-tracking`: the degradation requirement gains the condition it
  assumed - that "the tool is not installed" is reported when the tool is not
  installed, rather than when it merely is not on the app's inherited `PATH`.

## Impact

- `crates/knot-core/src/exec_path.rs` (new), `consts.rs`
  (`EXEC_PATH_FALLBACK_DIRS`), `lib.rs` - the shared roster and lookup.
- `crates/knot-agent-launch/src/adapter.rs`, `consts.rs` - `adapter_path` and
  `adapter_path_for` keep their signatures and behavior and delegate; the
  crate's own copy of the roster goes.
- `crates/knot-forge/src/runner.rs` - `GhRunner` locates `gh` and sets the
  child's `PATH`; `knot-forge` gains a `knot-core` dependency.
- No l10n change: the three availability messages already exist and still say
  the same things.
