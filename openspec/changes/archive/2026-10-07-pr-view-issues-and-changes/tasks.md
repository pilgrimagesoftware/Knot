# Tasks

> Implemented 2026-10-07 (#504). `make` is green. 8.2 was verified by the
> user in a running build on 2026-10-07 with two repos. The manual part of
> 4.2 (returning to the view keeps the tab) is checked here by unit test
> only. Notes: the Changes launcher row is now always shown - the delta's
> "a row labelled Changes is visible" has no exception for a workspace with
> no pull requests, and its issues and changes would otherwise be
> unreachable. `IssueFilter::is_active` was dropped as unused.

## 1. Repository resolution

- [x] 1.1 Add `knot-git` functions for a folder's top-level directory, its git common dir and a named remote's URL, each returning `None` outside a repository or without the remote; verify unit tests against a temp repo with a linked worktree and a folder outside git
- [x] 1.2 Add `knot-forge::github_repo(remote_url) -> Option<RepoSlug>` for the HTTPS, `git@` and `ssh://` forms, with and without `.git`; verify unit tests for each form and for a non-GitHub host
- [x] 1.3 Add `workspace_repos.rs` resolving distinct agent folders into `WorkspaceRepo`s keyed by common dir (worktrees merged, `label` = slug or top-level folder name); verify unit tests for two worktrees of one repo, a subfolder, a non-git folder and a non-GitHub remote

## 2. Issue fetching

- [x] 2.1 Add `knot-forge::issue_list_with(runner, slug, limit)` running `gh issue list --repo <slug> --state open --limit <limit+1> --json ...`, parsing into `Issue` and reporting `truncated`; add `ISSUE_LIST_LIMIT` to `consts.rs`; verify unit tests with a fake runner for the parsed fields, the truncation flag and a failed run
- [x] 2.2 Add the per-slug `IssueCache` on `WorkspaceWindow` with the five-minute max age, fetched in `spawn_blocking` per repository only while the Issues tab is shown, and wire it into `repaint_poll_tick`'s `if` chain; verify a unit test that a stale entry is claimed and a fresh one is not, and that the tick reports the cache as changed

## 3. OpenSpec change scanning

- [x] 3.1 Add `openspec_changes::scan(worktree)` skipping `archive` and dot-names and reading the first line of `## Why` from `proposal.md`; verify temp-dir unit tests for each scenario under "Un-archived changes are found in the workspace's folders"
- [x] 3.2 Merge scan results per `WorkspaceRepo` by name (first worktree kept) into a change-tracked cache with the 30-second max age, scanned in `spawn_blocking` only while the OpenSpec tab is shown, and wire it into `repaint_poll_tick`; verify a unit test for the merge and that a change in two worktrees yields one entry

## 4. Changes view and tabs

- [x] 4.1 Rename `WorkspaceViewMode::PullRequests` to `Changes`, the launcher row, view title and View menu entry to Changes, and gate pull request fetching on the Pull Requests tab; verify `make test` and that the existing pull request tests pass with the rename
- [x] 4.2 Add `ChangesTab` (closed enum with `Display`/`FromStr`) and `ChangesViewState` holding each tab's state, and render the tab bar in `render/changes_tabs.rs`; verify unit tests that each tab keeps its own search and a new window starts on Pull Requests, and manually that returning to the view keeps the tab
- [x] 4.3 Update `openspec/specs/pull-request-tracking` references in module docs and link the new capabilities from the new modules' docs; verify `grep` finds each spec path linked from its module

## 5. Issues tab

- [x] 5.1 Add `issue_filter.rs`: search (title, `#number`, `owner/repo`, URL, labels), repository filter with reset, the four sort orders within groups, grouping by slug; verify unit tests for each scenario under "The user can search, filter and sort the Issues tab"
- [x] 5.2 Add `render/issues_toolbar.rs` and `render/issues_pane.rs`: groups, rows (number, title, labels, updated age), truncation note, per-repository failure, gh missing/unauthenticated states, empty and filtered-empty messages, Refresh now and Copy URLs; verify pane tests for the message choice and `make size-check`
- [x] 5.3 Add the issue row click (open in browser), context menu and actions button with Open in browser and Copy URL, the button not opening the browser; verify with a click probe test like `tests/pull_request_row_clicks.rs`

## 6. OpenSpec tab

- [x] 6.1 Add search and repository filtering for changes; verify unit tests for the search-by-name scenario and the filter reset
- [x] 6.2 Add `render/openspec_toolbar.rs` and `render/openspec_pane.rs` with groups, rows (name, Why line), empty and filtered-empty messages and Refresh now; verify pane tests and `make size-check`
- [x] 6.3 Add the change row context menu and actions button with Reveal in Finder and Copy name; verify a test that Copy name yields the change name and Reveal targets the first worktree's directory

## 7. Send prompt to

- [x] 7.1 Extract `WorkspaceWindow::deliver_prompt(id, text)` and `can_receive_prompt(id)` from `broadcast_to_agents` and route broadcast through them; verify the existing broadcast and queueing tests pass
- [x] 7.2 Add `render/send_prompt_menu.rs` building the "Send prompt to" submenu from the workspace's agents (alphabetical, not-running agents disabled, item disabled with no agents) for a `WorkItemRef`, and add it to both menus of both tabs; verify unit tests for the ordering and disabled states
- [x] 7.3 Add the localized `changes_view.prompt.issue` / `changes_view.prompt.change` texts and send through `deliver_prompt` with the view left unchanged; verify a test that the chosen agent alone receives the substituted text and that a busy agent queues it

## 8. Localization and integration

- [x] 8.1 Add every new `changes_view.*`, `issues.*` and `openspec_changes.*` key and the renamed launcher/menu keys to `crates/knot-core/locales/en.yml`, touch `knot-core` so the build picks them up, and assert each resolves; verify `make test`
- [x] 8.2 Run `make` and manually verify against a workspace with two repositories (one with worktrees) that issues and changes are listed once per repository and that sending an issue and a change to an agent delivers the specified prompts - verified by the user in a running build on 2026-10-07 with two repos
