# Design

## Context

`pilgrimagesoftware/Knot` is public, AGPL-3.0, default branch `develop`, with
`develop` and `main` rulesets requiring the Rust CI matrix. Releases run
through `pilgrimagesoftware/github-actions` reusable workflows, which take the
repository from the Actions context rather than a hard-coded name. The name
is hard-coded in `knot_core::consts::KNOT_REPO` (the bug reporter's target),
`Cargo.toml` `repository`, `crates/knot/Cargo.toml` `homepage`, `README.md`,
several tests, and full URLs in archived OpenSpec changes.

GitHub redirects a renamed repository's old URLs - web, API, `git` - only
until a repository is created under the old name. Creating the meta-repository
`Knot` ends those redirects.

## Goals / Non-Goals

**Goals:**
- No window in which the app repository, its CI or its bug reporter points at
  the wrong repository.
- Each new repository usable from the day it is created: README, license,
  default branch and branch protection matching the app's.

**Non-Goals:**
- Tooling in the meta-repository beyond submodule wiring and a README.

## Decisions

### Order: rename, update, verify, then reuse the name

1. Rename `Knot` to `Knot-App` in the GitHub settings. Redirects keep old
   clones, links and released bug reporters working.
2. Land one PR in Knot-App updating every hard-coded reference. Its CI run is
   the proof that Actions, rulesets and the reusable release workflows survive
   the rename.
3. Update the `origin` remote of the primary checkout (worktrees share it).
4. Only then create the meta-repository `Knot`.

Alternative: create the meta-repository under another name (`Knot-Meta`) and
keep redirects forever. Rejected: the request is for the family's front door
to be `pilgrimagesoftware/Knot`.

### Released builds' bug reports

A released build files on `pilgrimagesoftware/Knot`, which becomes the
meta-repository. The meta-repository keeps issues enabled with a
`config.yml` issue template chooser whose only link sends reporters to
Knot-App's issues, and blank issues disabled - but `gh issue create`, which
the reporter uses, bypasses templates. The meta-repository's README and the
chooser say where app bugs belong, and the maintainer transfers stray issues
with `gh issue transfer`. Cut a release promptly after step 2 so the window
stays short.

Alternative: disable issues on the meta-repository. Rejected: `gh issue
create` would fail and the reporter would fall back to the browser compose
URL, which also fails; a transferable issue beats a lost report.

### Submodules use relative URLs and track `develop`

`.gitmodules` in the meta-repository:

```ini
[submodule "Knot-App"]
	path = Knot-App
	url = ../Knot-App.git
	branch = develop
[submodule "Knot-MCP"]
	path = Knot-MCP
	url = ../Knot-MCP.git
	branch = develop
[submodule "Knot-Library"]
	path = Knot-Library
	url = ../Knot-Library.git
	branch = main
```

Relative URLs resolve against the meta-repository's own remote, so an SSH
clone fetches submodules over SSH and an HTTPS clone over HTTPS. Dependabot's
`gitsubmodule` ecosystem opens a weekly PR advancing each pin, so the
meta-repository does not silently fall behind.

### New repositories' defaults

| Repository | License | Default branch | Contents at creation |
| --- | --- | --- | --- |
| Knot (meta) | AGPL-3.0 | `main` | README, LICENSE, `.gitmodules`, issue chooser, dependabot |
| Knot-MCP | AGPL-3.0 | `develop` (git-flow, like the app) | README, LICENSE, AGENTS.md, `openspec/` init |
| Knot-Library | CC BY 4.0 | `main` | README, LICENSE, `personas/.gitkeep` |

Knot-MCP follows the app's git-flow and ruleset shape because it will ship
software; its CI ruleset is added with its first code. Knot-Library holds
prompts and data rather than code, so a content license suits it better than
AGPL, and a single `main` branch is enough.

### Local checkouts stay where they are

The primary checkout keeps its path, `~/Code/ThirdParty/Knot`, so running
agents' folders and the `<checkout>-Worktrees` convention are unaffected; only
`origin`'s URL changes. The meta-repository is cloned separately when wanted.
Docs that say "a checkout at `~/Code/ThirdParty/Knot`" stay correct as
examples.

## Risks / Trade-offs

- [A collaborator's clone silently fetches the meta-repository after step 4]
  → Announce the rename; `git fetch` then fails on missing `develop` history
  rather than merging unrelated trees, which makes the problem visible.
- [Bug reports from released builds land on the meta-repository] → Issue
  chooser, README note, `gh issue transfer`; a release right after step 2.
- [External links (blog, release notes, homepage) break after step 4] →
  Search `pilgrimagesoftware.com` sources and the GitHub Release bodies for the
  old URL during step 2.
- [The reusable release workflows or package signing reference the name] →
  Step 2's CI run and a dry run of `package.yml` on the PR catch this before
  the name is reused.

## Migration Plan

Steps 1-4 above, then create Knot-MCP and Knot-Library and add all three
submodules. Rollback before step 4: rename `Knot-App` back to `Knot` and
revert the reference PR. After step 4, rollback means deleting or renaming the
meta-repository first.

## Open Questions

- Whether Knot-MCP later takes over the app's `knot-mcp` crate or depends on
  it. Decided by the MCP server's own design change.
