# Tasks

## 1. One shared search path

- [x] 1.1 Add `EXEC_PATH_FALLBACK_DIRS` to `knot-core`'s `consts.rs`, carrying
      the five locations `knot-agent-launch` already used, and say in the doc
      comment why there is one roster rather than one per crate
- [x] 1.2 Add `crates/knot-core/src/exec_path.rs` with `search_path`,
      `search_path_for`, `resolve_program` and `resolve_program_on`; resolve
      through symlinks and require the executable bit, since Homebrew installs
      every binary as a link into its Cellar
- [x] 1.3 Cover both in `exec_path/tests.rs`: ordering, dedup, `~` expansion
      against `HOME` and against an empty `HOME`, the launchd `PATH` case,
      first-directory-wins resolution, a non-executable file, an absent
      program, and a program that already names a file

## 2. `knot-agent-launch` uses it

- [x] 2.1 Delete the crate's `ADAPTER_PATH_FALLBACK_DIRS` and have
      `adapter_path`/`adapter_path_for` delegate to `knot-core`, keeping both
      signatures so `knot-terminal`'s ACP session path is untouched
- [x] 2.2 Drop the four merge tests now owned by `knot-core`, keeping the one
      that states this capability's own guarantee - the launchd `PATH` case

## 3. `GhRunner` locates `gh`

- [x] 3.1 Add `knot-core` to `knot-forge`'s dependencies and `tempfile` to its
      dev-dependencies
- [x] 3.2 Give `GhRunner` a `search_path` field, a `locate_gh` that falls back
      to the bare name, and an `.env("PATH", ...)` on the spawn; leave
      `with_program` verbatim so an explicit choice is not second-guessed
- [x] 3.3 Test in `runner/tests.rs`: `gh` in a directory the process `PATH`
      omits is located by absolute path, a path holding no `gh` falls back to
      the bare name (so Missing still reports), the located binary is what
      runs, and the child's `PATH` is the merged one

## 4. Gate

- [x] 4.1 `make` (fmt-check, size-check, lint, test, build)
- [x] 4.2 Launch the packaged app from Finder and open the pull request view on
      a workspace with a recorded pull request; confirm state is fetched and no
      "not installed" message appears
