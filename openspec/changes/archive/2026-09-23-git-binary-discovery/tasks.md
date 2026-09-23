# Tasks

## 1. `knot-git` accepts a git

- [x] 1.1 Add `GIT_PROGRAM` to `consts.rs` and `program.rs` holding a
      set-once `OnceLock<GitProgram>` with `configure` and `configured`; say
      in the module doc why the resolving is deliberately elsewhere
- [x] 1.2 Have `Runner::new` read the configured value, and `run` set the
      child's `PATH` when one was given
- [x] 1.3 Unit-test the unconfigured default; put the set-once behavior in
      `tests/program.rs`, one test in its own binary, because a
      process-wide value cannot be asserted twice in parallel

## 2. The binary resolves it

- [x] 2.1 Add `crates/knot/src/external_tools.rs` resolving git through
      `knot_core::exec_path` and configuring `knot-git`, with a pure
      `git_program` seam and tests for located, unlocated, and the search
      path being carried either way
- [x] 2.2 Call it from `main` before `app_bootstrap::run`

## 3. Gate

- [x] 3.1 `make`
- [x] 3.2 On a machine with no Xcode command line tools and a Homebrew git,
      confirm the git panel reads status. Not reproducible here - this
      machine has both - so it stands as the one claim these tests do not
      make
