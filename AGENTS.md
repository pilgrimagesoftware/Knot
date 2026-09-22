# Knot (Rust port) - Agent Development Guide

Context for AI agents working on this codebase.

## About This Project

Knot is a macOS app that runs a team of AI coding agents, each in an embedded
terminal, and lets them coordinate over MCP. It was built in Swift/SwiftUI; this
repo ports it to Rust.

Two trees live here:

- `crates/` - the Rust workspace (the port, in progress).
- `Skwad/`, `SkwadTests/`, `Package.swift`, `Skwad.xcodeproj` - the original
  Swift app. It is the **behavioral reference**: when porting a subsystem, its
  Swift implementation is the spec for what the Rust code must do. Do not add
  features to the Swift app here.

The port proceeds one subsystem at a time. Each is driven by an OpenSpec change
under `openspec/changes/` and pinned to a contract under `openspec/specs/`, which
the crate's module docs link back to.

## Workspace Layout

```
crates/
├── knot-core/        Shared types, error model, constants, localization (l10n::t)
├── knot-git/         Runs `git`, parses porcelain v2 status, unified diffs,
│                      numstat; staging/commit, branch + ahead/behind, worktrees.
│                      Runtime-agnostic (no async runtime; wrap in spawn_blocking).
├── knot-discovery/   Maps a source folder to repos + linked worktrees via
│                      filesystem reads only (no `git` process); tokio-driven
│                      debounced folder watch. `scan()` is a pure function.
└── knot/             Binary. GPUI Kit window shell (gpui-kit crate).
```

### Dependency relationships

- `knot` depends on `knot-core` and `gpui-kit` (external, the UI toolkit).
- `knot-git` and `knot-discovery` are standalone: they depend on `thiserror`
  (via workspace) and, for discovery, `notify` + `tokio`.
- Shared dep versions are pinned in the root `Cargo.toml` `[workspace.dependencies]`.
  Add new shared deps there, reference with `dep.workspace = true`.
- Edition 2024, `rust-version = "1.98"`, toolchain pinned by `rust-toolchain.toml`.

### Contracts (OpenSpec)

- `openspec/specs/repo-discovery/spec.md` - `knot-discovery`
- `openspec/specs/git-operations/spec.md` - `knot-git`
- `openspec/specs/worktree-management/spec.md` - `knot-git` worktree module

In-progress and archived changes are under `openspec/changes/`. Use the
`opsx:*` / OpenSpec skills to propose, apply, and archive changes.

## Committing Code

[Conventional Commits](https://www.conventionalcommits.org/). Scope is the crate
name (or `openspec`, `rust`, `ci`):

```
feat(knot-git): parse ahead/behind from status
fix(knot-discovery): debounce watch events per folder
docs(openspec): archive worktree-management-port change
build(rust): add dependabot config
```

PRs merge with a **merge commit** so the prefixes survive in history. Do not
squash.

## Branches and Workflow

git-flow. `develop` is the integration branch; `main` is release-only.

Always do code changes in a dedicated `git worktree` on its own feature
branch, no exceptions - never commit directly in the primary checkout (it
stays on `develop`/`main` for syncing and reference) and never commit
straight to `develop` or `main`. Use the `project-start-change` skill to
create the worktree.

Worktrees go in a peer directory beside the checkout, `<checkout>-Worktrees`,
one subdirectory per branch, named `<issue>-<change>` to match the branch. So
a checkout at `~/Code/ThirdParty/Knot` keeps them in
`~/Code/ThirdParty/Knot-Worktrees/<issue>-<change>`. Derive the root rather
than assuming a path - the checkout moves between machines, the convention
does not:

```bash
REPO=$(git rev-parse --show-toplevel)
git worktree add "$REPO-Worktrees/<issue>-<change>" -b <issue>-<change> origin/develop
```

A peer directory, never one inside the checkout, so worktree files don't show
up as untracked noise in the primary checkout. `git worktree list` is
authoritative for finding the existing ones.

1. Branch from `develop`: `feature/<change>` (or `release/x.y.z`, `hotfix/x.y.z`).
2. Implement against the OpenSpec change / spec contract.
3. Open a PR to `develop` (`release/*` and `hotfix/*` PR to `main`). Rulesets on
   both branches require the Rust CI matrix (`workspace (ubuntu-latest)` and
   `workspace (macos-latest)` from `.github/workflows/ci.yml`) to pass, and a
   PR to merge. Merge with a merge commit or rebase, never squash.
4. `dependabot` opens weekly grouped PRs against `develop` for `cargo` and
   `github-actions`.

### PR Preflight

- Before committing, confirm `git branch --show-current` is the dedicated feature branch and not `develop` or `main`.
- Before opening a PR, confirm the base is `develop` for feature work. `main` is only for release and hotfix PRs.
- Never open a feature PR from `develop` into `main`. If a commit landed on `develop` accidentally, stop and resolve the branch state before opening any PR.
- After opening a PR, verify its base and head with `gh pr view <number> --json baseRefName,headRefName,url`.

## Running Checks Locally

```bash
make             # the whole gate, in the order CI runs it

make fmt         # reformat with the pinned nightly
make fmt-check   # verify formatting (what CI runs)
make size-check  # fail on any .rs file over 700 lines
make lint        # cargo clippy --workspace --all-targets -- -D warnings
make test        # cargo test --workspace
make build       # cargo build --workspace
```

```bash
make package     # Knot.app + DMG via cargo-packager (macOS only)
```

Packaging is configured in `crates/knot/Cargo.toml` under
`[package.metadata.packager]` and needs `cargo install cargo-packager --locked`.
CI runs the same command from `.github/workflows/package.yml`.

Run `make fmt` before committing. `rustfmt.toml` uses unstable options, so
formatting is only reproducible on one exact nightly, pinned as
`RUSTFMT_NIGHTLY` in the `Makefile` and installed with `rustup toolchain install
$(make -s print-rustfmt-nightly)`. CI installs that same pin and runs these same
targets. `knot-git` tests need `git` >= 2.30 on `PATH` for porcelain v2.

The `Makefile` drives the Rust workspace only. It used to carry the Swift
app's xcodebuild targets as well - which is why the Rust ones were all prefixed
`rust-` - but those were removed along with the Swift release workflow they
fed. Build or test the Swift reference through `Skwad.xcodeproj` in Xcode.

## Releases

`prepare-release.yml` / `tag-release.yml` / `release.yml` automate the
git-flow release cycle (version bump + changelog via `cargo-edit`/`git-cliff`,
tag `main`, cut a GitHub Release, merge `main` back into `develop`), calling
the reusable `pilgrimagesoftware/github-actions` `rust-*-release` workflows, and
`package.yml` builds `Knot.app` with `cargo-packager`. Details and
required secrets: `CONTRIBUTING.md` - Releases. Rationale: `docs/adr/0001-git-flow.md`.

## Architecture Decisions

Non-trivial design choices are recorded as ADRs under `docs/adr/`. Index and
process: `docs/adr/README.md`. `/adr "<title>"` scaffolds a new record from
`docs/adr/0000-template.md`.

## Conventions

- **No `.rs` file over 700 lines.** Enforced by `make size-check` in CI. Split
  by concern, not by line count; move colocated tests to a sibling `tests.rs`
  first. Do not raise the limit to make a change fit.
- **`mod.rs` declares; it does not implement.** Module declarations,
  re-exports and the module doc comment only - implementation goes in sibling
  files named for what they do. Being under the line limit is not evidence the
  code is in the right place. The six `mod.rs` files that predate the rule are
  a backlog (issue #300), not exceptions.
- **No crate-wide `allow`.** Allow on the item, with a comment saying why.
  `UNWIRED` marks ported-but-unconnected code, `SUPERSEDED` marks code a newer
  path replaced - both greppable.
- Constants live in a single `consts.rs` per crate.
- Errors: `thiserror` enums per crate (`GitError`, `DiscoveryError`, core
  `Error`), re-exported with a crate `Result` alias.
- User-facing text goes through `knot_core::l10n::t`; tests assert the key
  resolves, never the English copy.
- Keep functions to <= 5-6 args; group related args in a struct.
- Closed vocabularies are enums with `Display`/`FromStr`, not `String` matched
  with a `_ => default` arm. An open one (`agent_type`) keeps its `String` and
  gets one roster - `knot_core::agent_type` - that every crate reads.
- Locks are `parking_lot::Mutex`: no poisoning, so no `.unwrap()` or
  `if let Ok(..)` around a guard. `tokio::sync::Mutex` only where a guard is
  held across an `.await` (`knot-acp`'s transport).
- No I/O on the render path - GPUI re-renders per keystroke. Diff stats go
  through `knot/src/diff_stats.rs`, which has caught this three times.
- No statement-hugging brace style; format with nightly `rustfmt`.

`.claude/rules/rust-structure.md` has the reasoning behind each of these, with
the defect that produced it. Read it before a refactor; every rule names the
cost of breaking it, so you can tell when it genuinely does not apply.
