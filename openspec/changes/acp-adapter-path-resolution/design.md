# Design

## Context

See proposal.md - Why. The adapter subprocess and its install command are
spawned from `crates/knot-terminal/src/acp_session/mod.rs`: `start_inner`'s
`build_command()` (the adapter) and `run_install` (the install method). Both
use `Command::new(bare_name)`, resolved via the process `PATH`. A
Finder-launched macOS app gets `/usr/bin:/bin:/usr/sbin:/sbin` and never the
user's shell `PATH`, so Homebrew/cargo/local/npm installs are invisible.

The adapter registry `crates/knot-agent-launch/src/adapter.rs` is the one
place that knows the adapter binaries and their install commands, so the
search-path policy is published there and consumed by `knot-terminal`.

## Goals / Non-Goals

- **Goals**: adapter binaries and install commands resolve when the app was
  not launched from a shell; the user's own `PATH` ordering wins when it is.
- **Non-goals**: no settings surface (a user-tunable PATH list is future
  work and needs a proposal of its own); no change to terminal launches;
  no attempt to source shell dotfiles or run adapters through a login shell.

## Decisions

### D1. Publish a merged adapter PATH from `knot-agent-launch`

A pure function builds the merged path from the process `PATH` plus a fixed
list of fallback directories:

```
/opt/homebrew/bin          # Apple Silicon Homebrew
/usr/local/bin             # Intel Homebrew / common local installs
~/.cargo/bin               # rustup/cargo tool installs
~/.local/bin               # pip --user / script installs
~/.npm-global/bin          # npm global with npm_config_prefix
```

`~`-prefixed entries expand against `HOME` (a GUI process keeps `HOME` even
though the shell env is gone). Order: process `PATH` entries first, then
fallbacks, deduplicated (a fallback already named in the process `PATH` is
not repeated). The process `PATH` wins so a developer launch from a shell
keeps its exact ordering; the fallbacks only add what the GUI is missing.

Application: `AcpSession::start_inner` sets `command.env("PATH", merged)` on
the adapter command; `run_install` sets the same on the install command.

Why a fixed list over alternatives:
- **Login-shell wrapper** (`/bin/zsh -l -c ...`): sources dotfiles and would
  catch every install location, but costs ~100ms per spawn, can block on
  interactive prompts/rbenv shims in the launchd env, and risks mangling
  adapter stdio. Overkill for named binaries that live in a handful of
  well-known dirs.
- **`launchctl getenv PATH`**: empty for Finder-launched apps (that is the
  bug); not a source of truth.
- **Scanning every tool's shim root (anyenv/nvm/volta)**: unbounded and
  version-shimmed dirs need their own env vars to resolve; out of scope.

### D2. Testable pure function + env-level test

The merge is a pure `fn adapter_path(process_path: &str, home: &str) -> String`
unit-tested in `knot-agent-launch`. `knot-terminal` gets one test that the
adapter command actually carries the merged path in its environment (the
fake-adapter pattern already there, extended to echo `$PATH`), proving the
application site rather than just the builder.

## Risks / Trade-offs

- [A future install location that is not in the fixed list still fails from
  a Finder launch] → The error message already names the missing program;
  users can relaunch from a terminal or `launchctl setenv PATH`. Extending
  the list is a one-line change; user-configurable paths would follow as a
  separate change.
- [npm global bin can live at a per-version node root (nvm/volta/nodenv)
  invisible to this list] → `~/.npm-global/bin` covers the conventional
  prefix; version-manager adapters are the same class of future work as the
  risk above.

## Migration Plan

Pure additive launch behavior; no schema/settings/preferences changes.
Rollback is reverting the commit. No data migration.

## Open Questions

None.