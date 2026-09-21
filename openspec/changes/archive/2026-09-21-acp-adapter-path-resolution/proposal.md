# Proposal

## Why

When Knot is launched from Finder (the shipped `Knot.app`), it inherits
launchd's minimal `PATH` (`/usr/bin:/bin:/usr/sbin:/sbin`) instead of the
user's shell PATH. ACP adapters are spawned by bare command name (`opencode`
`acp`, `claude-agent-acp`, `gemini --acp`, ...), and those binaries are
typically installed into Homebrew (`/opt/homebrew/bin`), `~/.cargo/bin`,
`~/.local/bin` or an npm global bin - none of which the GUI sees. The spawn
fails with "failed to spawn acp adapter: No such file or directory", so every
Panel-mode session is dead on arrival for a Finder-launched app even though
the same command works from a terminal.

## What Changes

- Adapter subprocesses and their install commands are spawned with a `PATH`
  that merges the process's own `PATH` with the standard non-sandbox install
  locations a GUI-launched macOS app normally misses.
- The merged search path prefers the user's own `PATH` entries (terminal
  launches and user-specified overrides keep their ordering) and appends the
  fallback directories, deduplicated, only where they do not already appear.
- `~`-prefixed fallback directories (`~/.cargo/bin`, `~/.local/bin`,
  `~/.npm-global/bin`) are resolved against `HOME`, which a GUI process still
  has even though the shell env is gone.

## Capabilities

### New Capabilities
- none

### Modified Capabilities
- `agent-launch-command`: adds the requirement that an ACP adapter's declared
  command is resolved against a search path that includes the standard
  install locations even when the app did not inherit a shell `PATH`, so a
  registered adapter found in those locations launches.

## Impact

- `crates/knot-agent-launch`: a `PATH`-merge helper plus the fixed directory
  list; pure function with unit tests.
- `crates/knot-terminal` (`acp_session`): set the merged `PATH` on the adapter
  command built in `AcpSession::start_inner` and on install commands run by
  `run_install`.
- No protocol, settings, or UI changes. Existing terminal launches are
  unaffected (they use absolute `/bin/...` shells).