# Tasks

## 1. Rename the app repository

- [ ] 1.1 Rename `pilgrimagesoftware/Knot` to `pilgrimagesoftware/Knot-App` (`gh repo rename Knot-App --repo pilgrimagesoftware/Knot`); verify `gh repo view pilgrimagesoftware/Knot-App --json name,defaultBranchRef` succeeds and `gh api repos/pilgrimagesoftware/Knot-App/rulesets` still lists `develop` and `main`
- [ ] 1.2 Set the primary checkout's `origin` to `git@github.com:pilgrimagesoftware/Knot-App.git`; verify `git fetch origin` succeeds and `git worktree list` worktrees resolve `origin/develop`

## 2. Update references in Knot-App

- [ ] 2.1 Change `KNOT_REPO` in `crates/knot-core/src/consts.rs` to `pilgrimagesoftware/Knot-App` and update the tests and fixtures in `crates/knot`, `crates/knot-forge` that name the repository; verify `make test`
- [ ] 2.2 Update `repository` in `Cargo.toml`, `homepage` in `crates/knot/Cargo.toml`, the README badges and clone command, and repository names in `AGENTS.md`, `CONTRIBUTING.md` and `.claude/skills/project-*`; verify `grep -rn "pilgrimagesoftware/Knot\b"` outside `target/` finds only intended meta-repository references
- [ ] 2.3 Rewrite full `github.com/pilgrimagesoftware/Knot/(issues|pull)/` URLs in `openspec/changes/archive/` to `Knot-App`; verify the same grep finds none there
- [ ] 2.4 Check the `pilgrimagesoftware/github-actions` reusable workflows and `.github/workflows/package.yml` for the literal repository name; verify by a green CI run of the PR and a `package.yml` run on it
- [ ] 2.5 Open, merge (merge commit) and release the reference PR through the git-flow release workflows; verify a GitHub Release exists on Knot-App whose build files a test bug report on Knot-App (then close that issue)

## 3. Create Knot-MCP and Knot-Library

- [ ] 3.1 Create public `pilgrimagesoftware/Knot-MCP` with an AGPL-3.0 LICENSE, README describing the cross-Knot, multi-user MCP server as planned, AGENTS.md, and `openspec init`; create `develop` as default branch alongside `main`; verify `gh repo view` shows it public with default branch `develop`
- [ ] 3.2 Create public `pilgrimagesoftware/Knot-Library` with a CC BY 4.0 LICENSE, README describing it as the home of importable personas and shared data (format to be defined), and `personas/.gitkeep`; verify `gh repo view` and that `main` holds the three files

## 4. Create the meta-repository

- [ ] 4.1 Confirm steps 1-3 are done and the Knot-App release is out; verify `gh repo view pilgrimagesoftware/Knot` redirects to Knot-App (the name is still free only as a redirect)
- [ ] 4.2 Create public `pilgrimagesoftware/Knot` (default branch `main`, AGPL-3.0) with a README introducing the three repositories and `git clone --recurse-submodules`; verify `gh repo view pilgrimagesoftware/Knot --json name` returns `Knot`, not `Knot-App`
- [ ] 4.3 Add the three submodules with the relative URLs and branches in design.md, and commit; verify a fresh `git clone --recurse-submodules` over HTTPS and over SSH checks out all three
- [ ] 4.4 Add `.github/ISSUE_TEMPLATE/config.yml` (blank issues disabled, link to Knot-App issues) and a `gitsubmodule` dependabot config; verify the issue chooser on GitHub shows the Knot-App link
- [ ] 4.5 Protect the meta-repository's `main` (pull request required, no force-push); verify with `gh api repos/pilgrimagesoftware/Knot/rulesets`

## 5. Announce

- [ ] 5.1 Broadcast the rename to the knot and update the repository description and homepage links on Knot-App and pilgrimagesoftware.com; verify the homepage links resolve to Knot-App
