# Design

## Context

The Pull Requests view is split across `workspace_window/pull_requests*.rs`
(recording, reading, toolbar state) and `workspace_window/render/pull_requests_*.rs`
(pane, toolbar, launcher row), with pure logic in `pull_request_filter.rs`,
`pull_request_groups.rs` and `pull_request_state.rs`. State is fetched through
`knot-forge` (`GhRunner`, which runs `gh` with no working directory and names
every subject by URL or `--repo`), off-thread via `runtime.spawn_blocking`,
into a change-tracked `refresh_cache` read in `repaint_poll_tick`.

Nothing in the workspace knows its repositories: a `Workspace` is a list of
agent IDs, and an agent carries only a `folder`. No code reads a git remote or
lists issues, and no code reads `openspec/`.

`WorkspaceWindow::broadcast_to_agents` already delivers one text to every
agent: `deliver_panel_prompt(id, text, PromptOrigin::User)` for a panel agent
(send or queue), else `session.send_text` for a terminal agent. gpui-kit's
`PopupMenu::submenu` backs the sidebar's "New from Bench" submenu, the
template for "Send prompt to".

## Goals / Non-Goals

**Goals:**
- Reuse the Pull Requests view's patterns (toolbar state per window, change-
  tracked caches, `spawn_blocking` fetches gated on the view being shown).
- Keep new logic pure and unit-tested without GPUI: remote parsing, change
  scanning, issue filtering and sorting.

**Non-Goals:**
- Watching `openspec/changes/` for file-system events. A 30-second max age
  plus Refresh now is enough for a list the user reads occasionally.
- Sharing one generic "work item" row type across the three tabs. The tabs'
  rows have different fields and actions; a common trait would be wider than
  any one use.

## Decisions

### Repository resolution: `knot-git` for the folder, `knot-forge` for GitHub

For each distinct agent folder, `knot-git` runs `git rev-parse
--show-toplevel` and `git rev-parse --git-common-dir` (to collapse worktrees
of one repository) and `git remote get-url origin`. `knot-forge` gains a pure
`github_repo(remote_url) -> Option<RepoSlug>` accepting
`https://github.com/o/r(.git)`, `git@github.com:o/r(.git)` and
`ssh://git@github.com/o/r(.git)`.

The result is a `WorkspaceRepo { common_dir, worktrees: Vec<PathBuf>, slug:
Option<RepoSlug>, label }`, keyed by common dir. Resolution runs in one
`spawn_blocking` job whenever the Issues or Changes tab fetches, so a moved or
new agent folder is picked up without extra invalidation.

Alternative: run `gh issue list` with the agent's folder as its working
directory and let `gh` infer the repository. Rejected: `GhRunner` deliberately
runs without a working directory, `gh`'s inference picks the upstream of a
fork unpredictably, and worktree deduplication still needs git.

### Issues: `gh issue list --repo`, one job per repository

`knot-forge::issue_list_with(runner, slug, limit)` runs `gh issue list --repo
<slug> --state open --limit <limit+1> --json
number,title,url,labels,author,createdAt,updatedAt` and reports `truncated`
when it receives `limit + 1`. `gh issue list` already excludes pull requests.
`ISSUE_LIST_LIMIT = 100` goes in `knot-forge/src/consts.rs`.

Results land in an `IssueCache` (a `refresh_cache` keyed by slug, entry =
`Ok(IssuePage) | Failed`), with the five-minute max age in `knot/src/consts.rs`.
The availability probe is the one the Pull Requests tab already caches.

### Changes: a pure scanner over working trees

`openspec_changes::scan(worktree: &Path) -> Vec<ChangeEntry>` reads
`openspec/changes/` with `std::fs::read_dir`, skips `archive` and dot-names,
and reads the first non-empty line after `## Why` in `proposal.md`. It is a
pure file-system function tested against a temp dir. The window merges
entries per `WorkspaceRepo` by name, keeping the first worktree path for
Reveal in Finder. The scan runs in `spawn_blocking` after repository
resolution and lands in a change-tracked cache with a 30-second max age.

It lives in the `knot` crate rather than `knot-discovery`, whose contract is
repository discovery; OpenSpec is an application concern.

### Tabs and per-window state

`PullRequestViewState` stays as is. A new `WorkViewState { tab: WorkTab,
pull_requests, issues: IssueViewState, changes: ChangeViewState }` holds each
tab's search entity and filters. `WorkTab` is a closed enum with
`Display`/`FromStr`. The launcher row and `WorkspaceViewMode::PullRequests`
are renamed to `Work`; the Pull Requests fetch gate becomes "view is Work and
tab is PullRequests".

### Single-agent delivery shared with broadcast

Extract `WorkspaceWindow::deliver_prompt(id, text) -> bool` from
`broadcast_to_agents` (panel first, terminal fallback) and have broadcast loop
over it. `can_receive_prompt(id)` (a panel session or a terminal session
exists) decides whether the submenu entry is enabled. The submenu is built by
one function taking the agent list and a `WorkItemRef::{Issue(url),
Change(name)}`, shared by both menus of both tabs.

### Rendering and file layout

New files under `workspace_window/render/`: `work_tabs.rs`, `issues_pane.rs`,
`issues_toolbar.rs`, `changes_pane.rs`, `changes_toolbar.rs`,
`send_prompt_menu.rs`. Pure logic: `issue_filter.rs`, `openspec_changes.rs`,
`workspace_repos.rs`. Each new cache goes into `repaint_poll_tick`'s `if`
chain, per `.claude/rules/rust-structure.md`.

## Risks / Trade-offs

- [Many repositories mean many `gh` calls] → Fetch only while the Issues tab
  is shown, cap at the five-minute max age, and run repositories concurrently.
- [A fork's `origin` is the fork, not upstream, so upstream issues are not
  listed] → Accepted; `origin` is where the agents push. Reading `upstream`
  can be a later change.
- [The prompt names a change, not the worktree holding it, so an agent in a
  different worktree may not see it] → Accepted per issue #504's text; the
  agent can find it or ask.
- [Localized prompt text is sent to agents] → Required by the localization
  convention; the English copy is what agents are tuned for, and tests assert
  only that the keys resolve.
- [Renaming `WorkspaceViewMode::PullRequests` touches menus and keymaps
  (View menu navigation)] → Rename in one commit; the View menu entry is
  renamed with it.

## Migration Plan

No persisted data changes. Only the view title, launcher label and View menu
entry change name.
