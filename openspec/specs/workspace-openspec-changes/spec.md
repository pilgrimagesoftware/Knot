# workspace-openspec-changes Specification

## Purpose
Lists the OpenSpec changes not yet archived in a workspace's repositories, in
the Changes view's OpenSpec tab, so the user can see the planned work and hand a
change to an agent.

## Requirements

### Requirement: Un-archived changes are found in the workspace's folders

Knot SHALL look for OpenSpec changes in the working tree of each workspace
repository and each of its worktrees that an agent of the workspace works in,
the repositories being those `workspace-issues` describes. A change SHALL be a
directory directly under `<working tree>/openspec/changes/` other than
`archive`, and its name SHALL be the directory's name. Directories whose names
start with `.` SHALL be ignored. Anything under `openspec/changes/archive/`
SHALL NOT be listed.

A change with the same name in several working trees of one repository SHALL be
listed once for that repository. A working tree with no `openspec/changes/`
directory SHALL contribute nothing and SHALL NOT be reported as an error.

Unlike issues, changes SHALL be found for every git repository, whatever its
remote, and for repositories with no remote at all. A change SHALL be grouped
under its repository's `<owner>/<repo>` when the repository is on GitHub, and
under the name of the repository's top-level folder otherwise.

Knot SHALL look only while the OpenSpec tab is shown: when it is first shown in
a window, when it is shown again more than 30 seconds after the last look, and
when the user chooses Refresh now. Looking SHALL NOT block the window.

The Swift reference has no such notion.

#### Scenario: Listing a change

- **WHEN** an agent's repository has `openspec/changes/add-login/` and
  `openspec/changes/archive/2026-09-01-old-work/`
- **THEN** the OpenSpec tab lists `add-login` and not `old-work`

#### Scenario: A change in two worktrees

- **WHEN** two agents work in two worktrees of one repository and both hold
  `openspec/changes/add-login/`
- **THEN** `add-login` is listed once under that repository

#### Scenario: A change only in a feature worktree

- **WHEN** only a linked worktree of a repository holds
  `openspec/changes/add-login/`, and an agent works in that worktree
- **THEN** `add-login` is listed under that repository

#### Scenario: A repository with no OpenSpec directory

- **WHEN** an agent's repository has no `openspec/changes/` directory
- **THEN** it contributes no rows and the tab shows no error

#### Scenario: A newly proposed change appears

- **WHEN** an agent creates `openspec/changes/add-export/` and the user chooses
  Refresh now in the OpenSpec tab
- **THEN** `add-export` is listed

### Requirement: The OpenSpec tab lists changes by repository

The OpenSpec tab SHALL group changes by repository, groups ordered by their
heading, rows ordered by change name. Each row SHALL show the change's name
and, when the change has a `proposal.md`, the first line of its `## Why`
section.

A workspace with no un-archived changes SHALL say so rather than showing an
empty list.

#### Scenario: A row shows the proposal's summary

- **WHEN** `add-login/proposal.md` has a `## Why` section beginning `Users
  cannot sign in with SSO.`
- **THEN** the `add-login` row shows that line under its name

#### Scenario: No changes

- **WHEN** no workspace repository has an un-archived change
- **THEN** the tab says there are no OpenSpec changes

### Requirement: The user can search and filter the OpenSpec tab

The OpenSpec tab SHALL offer a toolbar holding a search field, a repository
picker and a list actions menu with Refresh now.

While the search field holds text, the tab SHALL show only changes whose name
or repository heading contains it, ignoring case and leading and trailing
whitespace. The repository picker SHALL behave as the Issues tab's does. A
search or filter that hides every row SHALL say so, with a control that clears
them.

The search and filter SHALL be held per workspace window and SHALL NOT be
persisted.

#### Scenario: Searching by name

- **WHEN** the tab lists `add-login` and `add-export`, and the user types
  `LOGIN`
- **THEN** only `add-login` is shown

### Requirement: A change row offers its actions

A change row SHALL offer its actions both in a context menu, opened by a
secondary click, and in an actions menu, opened from a button on the row. Both
menus SHALL hold the same items: Reveal in Finder, Copy name, and the "Send
prompt to" submenu described in `work-item-prompts`. Reveal in Finder SHALL
reveal the change's directory in the first working tree it was found in.

#### Scenario: Copying a change's name

- **WHEN** the user chooses Copy name from the `add-login` row's actions menu
- **THEN** the clipboard holds `add-login`

#### Scenario: Revealing a change

- **WHEN** the user chooses Reveal in Finder on the `add-login` row
- **THEN** Finder shows the `add-login` directory selected
