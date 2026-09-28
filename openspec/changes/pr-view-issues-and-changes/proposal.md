# Proposal

Issue: #504

## Why

The Pull Requests view shows what a workspace's agents have produced, but not
what is waiting for them. The open issues of the repositories the agents work in,
and the OpenSpec changes not yet archived there, live in the browser and the
file system, and handing one to an agent means copying a link or a change name
into its prompt by hand.

## What Changes

- The Pull Requests view becomes the **Work** view, with three tabs: Pull
  Requests (today's list, unchanged), Issues and Changes. The sidebar launcher
  row is renamed to match; its open/merged/closed breakdown still counts pull
  requests only.
- **Workspace repositories:** the repositories the view reads are derived from
  the folders of the workspace's agents. Worktrees of one repository count as
  one repository.
- **Issues tab:** lists the open issues of every workspace repository whose
  `origin` remote is on GitHub, fetched through `gh`, grouped by repository.
  It has a search field, a repository filter, a sort order and Refresh now.
- **Changes tab:** lists the un-archived OpenSpec changes found under
  `openspec/changes/` in the workspace's agent folders, grouped by repository.
  A change present in several worktrees of one repository is listed once.
  It has a search field and a repository filter.
- **Send prompt to:** a submenu on each issue and change row, in both its
  context menu and a new per-row actions menu, that lists the workspace's
  agents. Choosing one sends that agent `Work on this issue: <issue URL>` or
  `Work on this OpenSpec change: <change name>`, through the same path as the
  agent's own prompt, so a busy agent queues it.
- Other row actions: Open in browser and Copy URL for an issue; Reveal in
  Finder and Copy name for a change.

## Non-Goals

- Acting on GitHub: nothing here creates, closes, assigns, labels or comments
  on an issue.
- Closed issues, and forges other than GitHub.
- Archived OpenSpec changes, and editing or archiving a change from the view.
- A "Send prompt to" item on pull request rows.
- Persisting the chosen tab, search, filters or sort across a restart.
- Letting the user edit the prompt text before it is sent.

## Capabilities

### New Capabilities

- `workspace-issues`: which repositories a workspace reads, how their open
  issues are fetched and refreshed, and the Issues tab's list, controls and
  row actions.
- `workspace-openspec-changes`: how un-archived OpenSpec changes are found in
  a workspace's folders, and the Changes tab's list, controls and row actions.
- `work-item-prompts`: the "Send prompt to" submenu, the prompt text it sends,
  and how it is delivered to the chosen agent.

### Modified Capabilities

- `pull-request-tracking`: the view and its launcher row become the Work view
  with tabs, the Pull Requests list being the first; the view's per-window
  state gains the chosen tab.

## Impact

- `crates/knot-git` - read a repository's top-level folder and a remote's URL.
- `crates/knot-forge` - parse a GitHub `owner/repo` from a remote URL; list a
  repository's open issues with `gh issue list --repo`.
- `crates/knot/src/` - a pure OpenSpec change scanner; workspace repository
  resolution; issue and change caches filled off-thread and wired into
  `repaint_poll_tick`; a single-agent prompt delivery helper shared with
  `broadcast_to_agents`.
- `crates/knot/src/workspace_window/render/` - a tab bar, the Issues and
  Changes panes, toolbars and row menus, in new sibling files so no file
  crosses 700 lines.
- `crates/knot-core/locales/en.yml` - new `work.*`, `issues.*`, `changes.*`
  keys; the launcher and view title keys change.
- No change to the persisted pull request records.
