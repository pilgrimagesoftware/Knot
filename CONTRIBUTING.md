# Contributing to Knot (Rust port)

This repo holds the Rust port of Knot alongside the original Swift/SwiftUI app.
The Swift app under `Skwad/` is the behavioral reference; the port moves it into
the `crates/` workspace one subsystem at a time, each driven by an OpenSpec
change and tied to a spec contract.

## Before you start

- Rust toolchain is pinned by `rust-toolchain.toml` (1.98.0, with `rustfmt` and
  `clippy`). `rustup` picks it up automatically.
- `git` >= 2.30 must be on `PATH` - `knot-git` tests parse porcelain v2 output.
- Nightly `rustfmt` is used for formatting: `rustup toolchain install nightly`.

## Workflow

git-flow. `develop` is the integration branch; `main` is release-only.

1. Branch from `develop`: `feature/<change>` (e.g. `feature/git-operations-port`).
   Release and hotfix branches (`release/x.y.z`, `hotfix/x.y.z`) PR to `main`.
2. For a subsystem port, work through its OpenSpec change under `openspec/changes/`.
   The spec files in `openspec/specs/` are the contract - crate module docs link
   back to them.
3. Keep commits scoped and conventional (see below).
4. Open a PR against `develop` (release/hotfix against `main`). CI
   (`.github/workflows/rust.yml`) must pass: `cargo fmt --check`,
   `cargo clippy -D warnings`, `cargo test`, `cargo build` on Linux and macOS.
5. PRs merge with a merge commit or rebase - never squash - so Conventional
   Commit prefixes survive in history.

## Running checks locally

```bash
make rust          # fmt (nightly) + clippy + test + build for the whole workspace

# or individually
make rust-fmt
make rust-lint
make rust-test
make rust-build
```

`make rust-fmt` runs `cargo +nightly fmt --check`. Run `cargo +nightly fmt`
before committing.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/). Use the crate name
as the scope:

```
feat(knot-git): parse ahead/behind from status
fix(knot-discovery): debounce watch events per folder
docs(openspec): archive worktree-management-port change
build(rust): add dependabot config
```

## Architecture decisions

Non-trivial design choices get an ADR under `docs/adr/`. Run `/adr "<title>"` or
copy `docs/adr/0000-template.md`. Index: `docs/adr/README.md`.

## Changelog

User-facing changes go under `## [Unreleased]` in `CHANGELOG.md` in the
Keep a Changelog format (Added / Changed / Fixed / Removed).

## Releases

Three workflows automate the git-flow release cycle for the Rust workspace,
calling the reusable `pilgrimagesoftware/github-actions` `rust-*-release` workflows:

1. **Prepare Release** (`prepare-release.yml`, manual dispatch) - bumps every
   crate's version with `cargo-edit`, regenerates `CHANGELOG.md` with
   `git-cliff`, and opens a `release/x.y.z` PR against `main`.
2. **Tag Release** (`tag-release.yml`) - fires when a `release/*` (or
   `hotfix/*`) PR merges into `main`; tags the merge commit `vX.Y.Z`.
3. **Release** (`release.yml`) - fires on that tag push; runs the workspace
   test suite, cuts a GitHub Release with `git-cliff`-generated notes, and
   merges `main` back into `develop` so the two branches stay in sync. It then
   calls **Package** for the tag and attaches the build to the release.

These need `PSW_CI_APP_ID` / `PSW_CI_PRIVATE_KEY`, organization secrets on
`pilgrimagesoftware` and available to this repo (a GitHub App
with `contents: write` + `pull-requests: write`, installed on this repo) before
Prepare/Tag Release can run - they mint a bot token so the release commit and
the develop merge-back satisfy branch protection. No crate here publishes to
crates.io (`publish-to-crates-io: false`).

**Package** (`package.yml`) builds the application itself with
[`cargo-packager`](https://docs.rs/cargo-packager), configured in
`crates/knot/Cargo.toml` under `[package.metadata.packager]` - the same
arrangement the other pilgrimagesoftware Rust apps use. It runs:

- on every merge into `develop`, keeping `Knot.app` and the DMG as a workflow
  artifact for 30 days;
- from `release.yml` for a tag, which also attaches
  `Knot_<version>_aarch64.dmg` to the GitHub Release.

macOS on Apple Silicon is the only target: the app links AppKit through
`objc2`. Signing and notarization happen when the Apple secrets are set -
`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`,
`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`, which cargo-packager reads from
the environment and cannot take from the manifest. Without them it packages
unsigned, which runs locally but is not distributable. Locally:
`make rust-package`, after `cargo install cargo-packager --locked`.

## License

By contributing you agree your work is licensed under AGPL-3.0-only, matching the
project.
