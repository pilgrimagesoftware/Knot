# Spec Delta

## Purpose

Lists the open GitHub issues of the repositories a workspace's agents work in,
in the Work view's Issues tab, so the user can see what is waiting and hand an
issue to an agent.

## ADDED Requirements

### Requirement: A workspace's repositories come from its agents' folders

A workspace's repositories SHALL be the git repositories containing the folders
of the workspace's agents. A folder SHALL resolve to the repository whose
working tree contains it, so an agent started in a subfolder counts toward its
repository. Worktrees of one repository SHALL count as one repository, however
many agents work in them.

A repository SHALL be identified on GitHub by the URL of its `origin` remote,
in either its HTTPS or its SSH form. A folder that is not in a git repository,
a repository with no `origin` remote, and a repository whose `origin` is not on
GitHub SHALL contribute no issues, and SHALL NOT be reported as an error.

Adding an agent to the workspace, removing one, or changing an agent's folder
SHALL change the workspace's repositories the next time the Issues tab fetches.

The Swift reference has no such notion.

#### Scenario: Two worktrees of one repository

- **WHEN** one agent works in a checkout of `acme/widget` and another in a
  linked worktree of the same repository
- **THEN** the Issues tab lists `acme/widget`'s issues once

#### Scenario: An SSH remote

- **WHEN** an agent's repository has `origin` set to
  `git@github.com:acme/widget.git`
- **THEN** the Issues tab lists `acme/widget`'s open issues

#### Scenario: A folder outside git

- **WHEN** one agent works in a folder that is not in a git repository and
  another works in `acme/widget`
- **THEN** the Issues tab lists `acme/widget`'s issues and shows no error for
  the other folder

#### Scenario: A remote not on GitHub

- **WHEN** the only agent's repository has `origin` on a host other than
  GitHub
- **THEN** the Issues tab says the workspace has no GitHub repositories

### Requirement: A repository's open issues are fetched and refreshed

Knot SHALL fetch each workspace repository's open issues through the `gh` tool,
found the way "The tool is found where it is installed" in
`pull-request-tracking` describes. Pull requests SHALL NOT be listed as issues.
For each issue Knot SHALL keep its number, title, URL, labels, author, and when
it was opened and last updated.

Knot SHALL fetch at most 100 open issues per repository, the most recently
updated first. When a repository has more, the tab SHALL say that only the
first 100 are shown for that repository.

Knot SHALL fetch only while the Issues tab is shown: when the tab is first
shown in a window, when it is shown again after its issues are more than five
minutes old, and when the user chooses Refresh now. Fetching SHALL NOT block
the window: the tab SHALL stay responsive while a fetch runs, and SHALL keep
showing the issues it last fetched until a new answer arrives. Each repository
SHALL be fetched independently, so a slow or failing repository does not hold
back the others.

Fetched issues SHALL NOT be persisted: a relaunched window fetches again.

#### Scenario: The first showing fetches

- **WHEN** the user opens the Issues tab for the first time in a window
- **THEN** each workspace repository's open issues are fetched and listed
  without the window freezing

#### Scenario: Old issues are refetched

- **WHEN** the user returns to the Issues tab six minutes after its last fetch
- **THEN** the issues are fetched again, and the previous list is shown until
  the answer arrives

#### Scenario: A pull request is not an issue

- **WHEN** a repository has two open issues and one open pull request
- **THEN** its group in the Issues tab has two rows

#### Scenario: More issues than the limit

- **WHEN** a repository has 150 open issues
- **THEN** its group shows 100 rows and says only the first 100 are shown

#### Scenario: One repository fails

- **WHEN** the fetch for `acme/gadget` fails and the fetch for `acme/widget`
  succeeds
- **THEN** `acme/widget`'s issues are listed and `acme/gadget`'s group says its
  issues could not be fetched

### Requirement: The Issues tab degrades rather than fails without the tool

When `gh` is not installed, or is installed but not signed in, the Issues tab
SHALL say which of the two it is and how to fix it, in place of the list,
rather than showing an empty list or an error dialog. The tab SHALL recheck
when the user chooses Refresh now.

#### Scenario: The tool is not installed

- **WHEN** `gh` cannot be found and the user opens the Issues tab
- **THEN** the tab says the GitHub CLI is not installed

#### Scenario: The tool is not signed in

- **WHEN** `gh` is installed but not authenticated and the user opens the
  Issues tab
- **THEN** the tab says the GitHub CLI is not signed in

### Requirement: The Issues tab lists issues by repository

The Issues tab SHALL group issues by repository, groups ordered by
`<owner>/<repo>`, each headed by `<owner>/<repo>`. Each row SHALL show the
issue's number, title and labels, and how long ago it was last updated.

A workspace with GitHub repositories but no open issues SHALL say so rather
than showing an empty list. A workspace with no GitHub repositories SHALL say
that instead.

#### Scenario: Two repositories

- **WHEN** the workspace's repositories are `acme/widget` and `acme/gadget`
  and both have open issues
- **THEN** the tab shows an `acme/gadget` group above an `acme/widget` group

#### Scenario: No open issues

- **WHEN** every workspace repository has no open issues
- **THEN** the tab says there are no open issues

### Requirement: The user can search, filter and sort the Issues tab

The Issues tab SHALL offer a toolbar holding a search field, a repository
picker, a sort picker and a list actions menu.

While the search field holds text, the tab SHALL show only issues whose title,
number written as `#<number>`, repository written as `<owner>/<repo>`, URL or a
label name contains it, ignoring case and leading and trailing whitespace. The
list SHALL narrow as the user types.

The repository picker SHALL list "All repositories" and each workspace
repository in order. Choosing one SHALL show only that repository's group. If
the chosen repository stops being a workspace repository, the picker SHALL
return to "All repositories".

The sort picker SHALL offer recently updated first (the default), newest
first, oldest first, and number. Sorting SHALL order rows within each
repository group.

The list actions menu SHALL offer Refresh now and Copy URLs; Copy URLs SHALL
copy the URLs of the rows currently shown, one per line.

A search or filter that hides every row SHALL say so, with a control that
clears them, instead of claiming there are no open issues.

The search, filter and sort SHALL be held per workspace window and SHALL NOT be
persisted, as the Pull Requests tab's are.

#### Scenario: Searching by label

- **WHEN** one issue is labelled `bug` and another `enhancement`, and the user
  types `BUG`
- **THEN** only the issue labelled `bug` is shown

#### Scenario: Filtering to one repository

- **WHEN** the user chooses `acme/widget` in the repository picker
- **THEN** only the `acme/widget` group is shown

#### Scenario: Sorting by number

- **WHEN** a group holds issues 7, 12 and 3 and the user chooses number
- **THEN** the group lists 3, 7 and 12

#### Scenario: A search that matches nothing

- **WHEN** the user types text no issue matches
- **THEN** the tab says no issues match, with a control that clears the search

### Requirement: An issue row offers its actions

An issue row SHALL offer its actions both in a context menu, opened by a
secondary click, and in an actions menu, opened from a button on the row. Both
menus SHALL hold the same items: Open in browser, Copy URL, and the "Send
prompt to" submenu described in `work-item-prompts`.

Clicking the row itself SHALL open the issue in the default browser. Opening
either menu SHALL NOT also open the issue.

#### Scenario: Opening an issue

- **WHEN** the user clicks an issue row
- **THEN** the issue's URL opens in the default browser

#### Scenario: The actions button does not open the issue

- **WHEN** the user clicks a row's actions button
- **THEN** the actions menu opens and the browser does not

#### Scenario: Copying an issue's URL

- **WHEN** the user chooses Copy URL from an issue row's context menu
- **THEN** the clipboard holds the issue's URL
