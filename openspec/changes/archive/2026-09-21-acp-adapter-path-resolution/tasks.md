# Tasks

## 1. PATH merge helper (knot-agent-launch)

- [x] 1.1 Add `adapter_path(process_path: &str, home: &str) -> String` (and a
      no-arg `adapter_path()` reading the process env) to
      `crates/knot-agent-launch/src/adapter.rs`, merging the process `PATH`
      with the fixed fallback dirs (`/opt/homebrew/bin`, `/usr/local/bin`,
      `~/.cargo/bin`, `~/.local/bin`, `~/.npm-global/bin`), `~`-expanded
      against `home`, deduplicated, process entries first
- [x] 1.2 Add unit tests in `knot-agent-launch`: process-first ordering,
      dedup of an already-present fallback, `~` expansion, and a GUI-launch
      case (`/usr/bin:/bin:/usr/sbin:/sbin` + HOME) that yields the
      fallbacks appended; verify `cargo test -p knot-agent-launch` passes

## 2. Apply the merged PATH at spawn sites (knot-terminal)

- [x] 2.1 Set the merged PATH on the adapter command built in
      `AcpSession::start_inner`'s `build_command()` via
      `command.env("PATH", knot_agent_launch::adapter_path())`
- [x] 2.2 Set the merged PATH on install commands in `run_install`
- [x] 2.3 Add a `knot-terminal` test that a spawned adapter process observes
      the merged PATH (extend the fake-adapter pattern to emit the value of
      `$PATH` and assert it contains a fallback dir); verify
      `cargo test -p knot-terminal acp_session` passes

## 3. Verify

- [x] 3.1 `make rust-fmt` (nightly) is clean on the change
- [x] 3.2 `make rust-lint` (clippy -D warnings) passes
- [x] 3.3 `make rust-test` passes for the whole workspace
- [x] 3.4 `openspec validate` passes for the change