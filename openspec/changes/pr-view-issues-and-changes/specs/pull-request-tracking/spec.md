# Spec Delta

## MODIFIED Requirements

### Requirement: The workspace window lists its pull requests

The workspace window SHALL offer a Work view, reached from a launcher row
labelled Work in the agent sidebar shaped like the Dashboard row described in
`dashboard`, and shown in place of the window's content the way the dashboard
view is. The Work view's Pull Requests tab SHALL list the pull requests
recorded against that workspace's agents; this capability's references to the
Pull Requests view mean that tab.

The list SHALL group rows by the agent that opened them. Within each group,
rows SHALL be ordered by the sort order the user has chosen, as "The user can
sort the Pull Requests view" describes; until the user chooses one, that order
is newest first, so the most recent work is at the top.

A pull request SHALL be listed once, however many of the workspace's agents
recorded it. Attribution stays per agent, as "A pull request URL in an agent's
output is recorded" requires; only the reading collapses. A pull request that
one agent recorded SHALL be listed in that agent's group. A pull request that
several agents recorded SHALL be listed in a group headed by all of them, and
pull requests recorded by the same agents SHALL share that group. Groups headed
by several agents SHALL come before single-agent groups, so a pull request that
several agents worked on is not hunted for. A row listed for several agents
SHALL be ordered by the earliest time any of them first saw it.

A workspace with no recorded pull requests SHALL say so rather than showing an
empty list.

The launcher row SHALL count pull requests only - not issues or OpenSpec
changes - and SHALL count each pull request once, however many agents
recorded it, and SHALL show how many of the workspace's recorded pull requests
are open, how many are merged and how many are closed, so the state of the
session's output is readable without opening the view. A draft pull request
SHALL count as open, matching the forge's own vocabulary, in which draft is a
property of an open pull request rather than a fourth state.

The launcher row SHALL count the workspace's records, not the rows the view is
currently showing: a search or filter in the view SHALL NOT change it.

A state Knot has not fetched SHALL NOT be guessed at. Where no state is known -
before the view has been shown for the first time, or where the tool that reads
state is unavailable - the row SHALL show the total count instead. Where some
are known and others are not, the row SHALL show the states it knows and count
the rest as pending, rather than folding them into any state. A pull request
the forge says does not exist SHALL be counted as not found - neither as a state
nor as pending - and the not-found count SHALL be shown only when it is not
zero.

The row SHALL show a selected background while the Work view is the one being
shown, whichever of its tabs is chosen.

#### Scenario: The launcher sits in the sidebar

- **WHEN** a workspace window is open
- **THEN** a row labelled Work is visible in the agent sidebar, shaped like
  the Dashboard row

#### Scenario: Showing the list

- **WHEN** the user clicks the Work row for the first time in a window
- **THEN** the window's content is replaced by the Work view on its Pull
  Requests tab, grouped by agent, newest first
- **AND** clicking the row again returns to the previous view

#### Scenario: An empty workspace

- **WHEN** no agent in the workspace has opened a pull request
- **THEN** the view says there are none rather than showing an empty list

#### Scenario: The count reflects the records

- **WHEN** an agent opens a second pull request while the workspace window is
  open
- **THEN** the launcher row's count becomes 2 without the user reopening the
  window

#### Scenario: The breakdown reflects the fetched states

- **WHEN** the workspace has four recorded pull requests whose states have been
  fetched - two open, one of them draft, one merged and one closed
- **THEN** the launcher row shows two open, one merged and one closed

#### Scenario: A state changes while the window is open

- **WHEN** a listed pull request is merged on GitHub and the next refresh runs
- **THEN** the launcher row's breakdown moves that one from open to merged
  without the user reopening the window

#### Scenario: No state has been fetched yet

- **WHEN** the workspace window is opened after a restart and the Work view's
  Pull Requests tab has not been shown
- **THEN** the launcher row shows the total count of records rather than a
  breakdown

#### Scenario: Only some states are known

- **WHEN** three pull requests have state and one request failed
- **THEN** the launcher row shows the three by state and counts the fourth as
  pending

#### Scenario: A pull request several agents opened

- **WHEN** two agents in the workspace have both recorded the same pull request
- **THEN** the list shows it once, in a group headed by both agents, above the
  single-agent groups
- **AND** the launcher row counts it once

#### Scenario: A pull request that does not exist is counted apart

- **WHEN** three pull requests are open and the forge says a fourth does not
  exist
- **THEN** the launcher row shows three open and one not found, and counts none
  as pending

#### Scenario: A filter does not change the launcher row

- **WHEN** the workspace has three open and two merged pull requests and the
  user filters the view to merged
- **THEN** the view shows two rows and the launcher row still shows three open
  and two merged

## ADDED Requirements

### Requirement: The Work view is divided into tabs

The Work view SHALL show a tab bar above its content with three tabs, in this
order: Pull Requests, Issues and Changes. Choosing a tab SHALL replace the
content below the bar with that tab's list and toolbar. Each tab SHALL keep its
own search, filters and sort, so switching tabs neither clears nor shares
them.

The chosen tab SHALL be held per workspace window, like the view's search,
filters and sort: leaving the view and returning to it SHALL show the same tab,
and a relaunched window SHALL open the view on Pull Requests.

Fetching SHALL follow the tab being shown: pull request state is fetched only
while the Pull Requests tab is shown, as "A recorded pull request's state is
fetched and refreshed" requires of the view, and the Issues and Changes tabs
fetch as `workspace-issues` and `workspace-openspec-changes` describe.

The Swift reference has no such view.

#### Scenario: Switching tabs

- **WHEN** the Work view is on Pull Requests and the user chooses Issues
- **THEN** the issues list and its toolbar replace the pull request list

#### Scenario: Each tab keeps its own search

- **WHEN** the user searches the Pull Requests tab for `login`, switches to
  Issues and back
- **THEN** the Issues tab's search field is empty and the Pull Requests tab's
  still holds `login`

#### Scenario: Returning to the view keeps the tab

- **WHEN** the user chooses Changes, switches to an agent's pane and clicks the
  Work row again
- **THEN** the view opens on Changes

#### Scenario: A relaunch opens on Pull Requests

- **WHEN** the view was last on Issues and Knot is quit and relaunched
- **THEN** the view opens on Pull Requests
