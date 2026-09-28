# Proposal

## Why

Knot is growing past one application. A cross-Knot, multi-user MCP server and
a library of shareable agent personas are each their own project, with their
own release cycle and contributors, but they belong to the Knot family. Keeping
them in the app repository would tie their history, CI and releases to the
app's. Giving each its own repository, gathered under a meta-repository named
`Knot`, keeps them separate while leaving one place to check out everything.

## What Changes

- **Rename** `pilgrimagesoftware/Knot` to `pilgrimagesoftware/Knot-App`.
  Issues, pull requests, releases, rulesets, secrets and Actions history move
  with it.
- **Update every reference** in the app repository to the new name before the
  old name is reused: the bug reporter's target repository
  (`knot_core::consts::KNOT_REPO`), the `repository` and `homepage` URLs in
  `Cargo.toml`, README badges and clone instructions, contributor docs, and
  full issue URLs in `openspec/`.
- **Create** `pilgrimagesoftware/Knot-MCP`, an empty-but-buildable home for a
  distinct MCP server shared across Knot installations and users.
- **Create** `pilgrimagesoftware/Knot-Library`, a home for agent personas and
  other shareable data that users can import into Knot.
- **Create** the meta-repository `pilgrimagesoftware/Knot` holding
  `Knot-App`, `Knot-MCP` and `Knot-Library` as git submodules, with a README
  describing the family and how to clone it.
- **Point local clones** at the new name: the primary checkout's and every
  worktree's `origin` remote.

**BREAKING** for anyone with a clone of the app: once the meta-repository takes
the name `Knot`, GitHub stops redirecting `pilgrimagesoftware/Knot` to the app,
so an un-updated clone fetches the meta-repository. Released builds of Knot
that file bug reports would file them on the meta-repository.

## Non-Goals

- Designing or building the multi-user MCP server. Knot-MCP starts with a
  README, license and scaffolding only; its behavior is a later change in that
  repository.
- Defining the Knot-Library data format, or adding import from it to the app.
  Both are later changes; the app's `personas` and `data-import` capabilities
  are untouched here.
- Moving the app's local checkout or its worktrees on disk. Agents in running
  workspaces have those paths as their folders.
- Moving code between repositories. The app's local `knot-mcp` crate stays in
  Knot-App.
- Combined CI or releases in the meta-repository.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. `bug-reporting` already files on "the app's repository"; only the value
of that repository's name changes. This change sets `skip_specs: true`.

## Impact

- GitHub: one rename, three new public repositories, submodule wiring.
- `crates/knot-core/src/consts.rs` - `KNOT_REPO` becomes
  `pilgrimagesoftware/Knot-App`; tests that name the repository follow.
- `Cargo.toml`, `crates/knot/Cargo.toml` - `repository` and `homepage`.
- `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `.claude/skills/project-*` -
  repository name and links.
- `openspec/changes/archive/*` - full issue and pull request URLs.
- Released builds before this change file bug reports on the meta-repository;
  the meta-repository's issue template redirects reporters to Knot-App.
