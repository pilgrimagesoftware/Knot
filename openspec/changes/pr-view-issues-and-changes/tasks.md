# Tasks

## 1. Repository resolution

- [ ] 1.1 Add `knot-git` functions for a folder's top-level directory, its git common dir and a named remote's URL, each returning `None` outside a repository or without the remote; verify unit tests against a temp repo with a linked worktree and a folder outside git
- [ ] 1.2 Add `knot-forge::github_repo(remote_url) -> Option<RepoSlug>` for the HTTPS, `git@` and `ssh://` forms, with and without `.git`; verify unit tests for each form and for a non-GitHub host
- [ ] 1.3 Add `workspace_repos.rs` resolving distinct agent folders into `WorkspaceRepo`s keyed by common dir (worktrees merged, `label` = slug or top-level folder name); verify unit tests for two worktrees of one repo, a subfolder, a non-git folder and a non-GitHub remote

## 2. Issue fetching

- [ ] 2.1 Add `knot-forge::issue_list_with(runner, slug, limit)` running `gh issue list --repo <slug> --state open --limit <limit+1> --json ...`, parsing into `Issue` and reporting `truncated`; add `ISSUE_LIST_LIMIT` to `consts.rs`; verify unit tests with a fake runner for the parsed fields, the truncation flag and a failed run
- [ ] 2.2 Add the per-slug `IssueCache` on `WorkspaceWindow` with the five-minute max age, fetched in `spawn_blocking` per repository only while the Issues tab is shown, and wire it into `repaint_poll_tick`'s `if` chain; verify a unit test that a stale entry is claimed and a fresh one is not, and that the tick reports the cache as changed

## 3. OpenSpec change scanning

- [ ] 3.1 Add `openspec_changes::scan(worktree)` skipping `archive` and dot-names and reading the first line of `## Why` from `proposal.md`; verify temp-dir unit tests for each scenario under "Un-archived changes are found in the workspace's folders"
- [ ] 3.2 Merge scan results per `WorkspaceRepo` by name (first worktree kept) into a change-tracked cache with the 30-second max age, scanned in `spawn_blocking` only while the Changes tab is shown, and wire it into `repaint_poll_tick`; verify a unit test for the merge and that a change in two worktrees yields one entry

## 4. Work view and tabs

- [ ] 4.1 Rename `WorkspaceViewMode::PullRequests` to `Work`, the launcher row, view title and View menu entry to Work, and gate pull request fetching on the Pull Requests tab; verify `make test` and that the existing pull request tests pass with the rename
- [ ] 4.2 Add `WorkTab` (closed enum with `Display`/`FromStr`) and `WorkViewState` holding each tab's state, and render the tab bar in `render/work_tabs.rs`; verify unit tests that each tab keeps its own search and a new window starts on Pull Requests, and manually that returning to the view keeps the tab
- [ ] 4.3 Update `openspec/specs/pull-request-tracking` references in module docs and link the new capabilities from the new modules' docs; verify `grep` finds each spec path linked from its module

## 5. Issues tab

- [ ] 5.1 Add `issue_filter.rs`: search (title, `#number`, `owner/repo`, URL, labels), repository filter with reset, the four sort orders within groups, grouping by slug; verify unit tests for each scenario under "The user can search, filter and sort the Issues tab"
- [ ] 5.2 Add `render/issues_toolbar.rs` and `render/issues_pane.rs`: groups, rows (number, title, labels, updated age), truncation note, per-repository failure, gh missing/unauthenticated states, empty and filtered-empty messages, Refresh now and Copy URLs; verify pane tests for the message choice and `make size-check`
- [ ] 5.3 Add the issue row click (open in browser), context menu and actions button with Open in browser and Copy URL, the button not opening the browser; verify with a click probe test like `tests/pull_request_row_clicks.rs`

## 6. Changes tab

- [ ] 6.1 Add search and repository filtering for changes; verify unit tests for the search-by-name scenario and the filter reset
- [ ] 6.2 Add `render/changes_toolbar.rs` and `render/changes_pane.rs` with groups, rows (name, Why line), empty and filtered-empty messages and Refresh now; verify pane tests and `make size-check`
- [ ] 6.3 Add the change row context menu and actions button with Reveal in Finder and Copy name; verify a test that Copy name yields the change name and Reveal targets the first worktree's directory

## 7. Send prompt to

- [ ] 7.1 Extract `WorkspaceWindow::deliver_prompt(id, text)` and `can_receive_prompt(id)` from `broadcast_to_agents` and route broadcast through them; verify the existing broadcast and queueing tests pass
- [ ] 7.2 Add `render/send_prompt_menu.rs` building the "Send prompt to" submenu from the workspace's agents (alphabetical, not-running agents disabled, item disabled with no agents) for a `WorkItemRef`, and add it to both menus of both tabs; verify unit tests for the ordering and disabled states
- [ ] 7.3 Add the localized `work.prompt.issue` / `work.prompt.change` texts and send through `deliver_prompt` with the view left unchanged; verify a test that the chosen agent alone receives the substituted text and that a busy agent queues it

## 8. Localization and integration

- [ ] 8.1 Add every new `work.*`, `issues.*` and `changes.*` key and the renamed launcher/menu keys to `crates/knot-core/locales/en.yml`, touch `knot-core` so the build picks them up, and assert each resolves; verify `make test`
- [ ] 8.2 Run `make` and manually verify against a workspace with two repositories (one with worktrees) that issues and changes are listed once per repository and that sending an issue and a change to an agent delivers the specified prompts
