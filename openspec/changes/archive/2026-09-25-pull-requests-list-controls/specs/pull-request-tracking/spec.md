# Spec Delta

## MODIFIED Requirements

### Requirement: The workspace window lists its pull requests

The workspace window SHALL offer a Pull Requests view listing the pull requests
recorded against that workspace's agents, reached from a launcher row in the
agent sidebar shaped like the Dashboard row described in `dashboard`, and shown
in place of the window's content the way the dashboard view is.

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

The launcher row SHALL count each pull request once, however many agents
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

The row SHALL show a selected background while the Pull Requests view is the one
being shown.

#### Scenario: The launcher sits in the sidebar

- **WHEN** a workspace window is open
- **THEN** a labelled Pull Requests row is visible in the agent sidebar,
  shaped like the Dashboard row

#### Scenario: Showing the list

- **WHEN** the user clicks the Pull Requests row
- **THEN** the window's content is replaced by the list, grouped by agent,
  newest first
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

- **WHEN** the workspace window is opened after a restart and the Pull Requests
  view has not been shown
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

### Requirement: A pull request the forge does not know is shown as not found

When the forge answers that a recorded pull request does not exist - its
repository cannot be resolved, or the repository has no pull request with that
number - Knot SHALL show that row as not found, with its own icon and wording,
rather than as pending. Not found is a finished answer; pending means no answer
has arrived.

The row SHALL still list its URL, SHALL still open in a browser, and SHALL still
be removable. Knot SHALL NOT remove a not-found record on its own: a private
repository the tool's identity cannot read gets the same answer as one that does
not exist, so removing it would be guessing with the user's data.

Once a pull request has been found not to exist, Knot SHALL NOT fetch its state
again while that window is open, unless the user asks for a refresh as "The
user can act on the whole list" describes. The answer is not persisted, so it
is asked again after a restart - which is also what recovers a repository that
becomes readable once the tool is authenticated for it. A manual refresh
recovers it without the restart.

The answer SHALL be recognised from the forge's own message, in one place, so
that every other failure - a timeout, a network error, an unparseable response
- stays a failed fetch that is retried.

#### Scenario: A repository that cannot be resolved

- **WHEN** a recorded pull request's repository does not exist, or cannot be
  read by the tool's identity
- **THEN** its row says the pull request was not found, with an icon of its own,
  and is neither pending nor any state

#### Scenario: A number the repository does not have

- **WHEN** a recorded pull request's repository exists but has no pull request
  with that number
- **THEN** its row says the pull request was not found

#### Scenario: Not found stops the refreshes

- **WHEN** a recorded pull request has been found not to exist and the view
  stays open past several refresh cycles
- **THEN** its state is not fetched again

#### Scenario: A restart asks again

- **WHEN** Knot restarts and the view is shown
- **THEN** a pull request previously found not to exist is fetched again, and
  shows its state if the forge now answers with one

#### Scenario: A manual refresh asks again

- **WHEN** a row has been found not to exist and the user chooses Refresh now
- **THEN** its state is fetched again, and the row shows its state if the forge
  now answers with one

#### Scenario: A not-found row stays the user's to remove

- **WHEN** a row has been found not to exist
- **THEN** it is still listed, still opens in a browser when clicked, and is
  removed only when the user removes it

#### Scenario: Other failures still retry

- **WHEN** a fetch times out or its response cannot be parsed
- **THEN** the row is not shown as not found, and a later refresh fetches it
  again

## ADDED Requirements

### Requirement: The view's search, filters and sort belong to the window

The Pull Requests view SHALL show a toolbar above its groups holding the search
field, the status and agent filters, the sort order and the list actions.

The search text, the filters and the sort order SHALL be held per workspace
window for as long as that window is open. Leaving the view and returning to it
SHALL keep them. They SHALL NOT be persisted: a relaunched window opens the view
with no search, no filter and the newest-first order.

Searching, filtering and sorting SHALL decide only which rows are drawn and in
what order. They SHALL NOT change which pull requests have their state fetched:
a row hidden by a filter is still refreshed, because the launcher row counts it
and expiry depends on it.

The toolbar SHALL be shown only when the workspace has at least one recorded
pull request.

The Swift reference has no pull request list, so it has nothing of the kind.

#### Scenario: Leaving and returning keeps the view's state

- **WHEN** the user searches for `widget`, switches to an agent's pane and
  opens the Pull Requests view again
- **THEN** the search field still holds `widget` and the list is still narrowed
  by it

#### Scenario: A relaunch starts clean

- **WHEN** the user has filtered the view to merged and Knot is quit and
  relaunched
- **THEN** the view opens with no filter, no search and newest first

#### Scenario: A hidden row is still refreshed

- **WHEN** the view is filtered to closed and an open pull request that is
  hidden by the filter is merged on GitHub
- **THEN** the launcher row's breakdown moves it from open to merged without
  the user clearing the filter

### Requirement: The user can search the Pull Requests view

The toolbar SHALL offer a search field. While it holds text, the view SHALL show
only the rows whose title, number written as `#<number>`, repository written as
`<owner>/<repo>`, URL, or opening agent's name contains that text. Matching
SHALL ignore case and SHALL ignore leading and trailing whitespace in the
search text. A field holding only whitespace SHALL be treated as empty.

A row whose state has not been fetched SHALL still be matched on what it has -
its URL, repository, number as parsed from its URL, and agent names - so a
search does not hide a row only because its title has not arrived.

The list SHALL narrow as the user types, without a separate submit.

#### Scenario: Searching by title

- **WHEN** the list holds pull requests titled `Fix login redirect` and
  `Add billing export`, and the user types `LOGIN`
- **THEN** only `Fix login redirect` is shown

#### Scenario: Searching by number

- **WHEN** the user types `#42`
- **THEN** a row for pull request 42 is shown

#### Scenario: Searching by repository

- **WHEN** the list holds pull requests from `acme/widget` and `acme/gadget`
  and the user types `acme/widget`
- **THEN** only the `acme/widget` rows are shown

#### Scenario: Searching by agent

- **WHEN** the user types the name of one of the workspace's agents
- **THEN** every row that agent opened is shown, including rows it shares with
  other agents

#### Scenario: A row without state still matches

- **WHEN** a row's state has not been fetched and the user types part of its
  repository name
- **THEN** that row is shown

#### Scenario: Clearing the search

- **WHEN** the user empties the search field
- **THEN** every row is shown again

### Requirement: The user can filter the Pull Requests view by status

The toolbar SHALL offer one toggle per status: open, merged, closed, not found
and pending. These are the launcher row's categories, so a draft pull request
SHALL be open for filtering, and a row whose fetch failed SHALL be pending.

With no toggle selected, the view SHALL show rows of every status. With one or
more selected, it SHALL show only rows whose status is among them.

Each toggle SHALL show how many rows have that status among the rows the search
and the agent filter leave, so the user can see what a toggle will show before
choosing it. A status filter SHALL combine with the search and the agent filter:
a row is shown only when it passes all three.

#### Scenario: Showing only open pull requests

- **WHEN** the list holds two open rows, one of them draft, and one merged row,
  and the user selects the open toggle
- **THEN** both open rows are shown and the merged row is not

#### Scenario: Two statuses at once

- **WHEN** the user selects the closed and not found toggles
- **THEN** rows that are closed or not found are shown, and no others

#### Scenario: Deselecting every toggle

- **WHEN** the user deselects the last selected toggle
- **THEN** rows of every status are shown

#### Scenario: A toggle's count follows the search

- **WHEN** the list holds three merged rows, one of them from `acme/widget`,
  and the user searches for `acme/widget`
- **THEN** the merged toggle shows a count of one

#### Scenario: A row changes status under a filter

- **WHEN** the view is filtered to open and a shown row's pull request is merged
  on GitHub
- **THEN** after the next refresh the row is no longer shown, and the merged
  toggle's count grows by one

### Requirement: The user can filter the Pull Requests view by agent

The toolbar SHALL offer an agent picker listing "All agents" and, in
alphabetical order, each of the workspace's agents that has at least one listed
pull request. Choosing an agent SHALL show only the rows that agent is among the
openers of, including rows in groups it shares with other agents. Group
headings SHALL be unchanged: a shared group is still headed by all of its
agents.

If the chosen agent is removed, or no longer has any listed pull request, the
picker SHALL return to "All agents" rather than leaving the view empty for a
reason the user can no longer see.

#### Scenario: One agent's pull requests

- **WHEN** agents Ada and Bo each opened one pull request and together opened a
  third, and the user chooses Ada
- **THEN** Ada's pull request and the shared one are shown, the shared one still
  under a heading naming both

#### Scenario: The chosen agent goes away

- **WHEN** the view is filtered to an agent and that agent is removed
- **THEN** the picker shows "All agents" and every remaining row is shown

### Requirement: The user can sort the Pull Requests view

The toolbar SHALL offer a sort picker with these orders, applied to the rows
within each group:

- **Newest first** - by the earliest time any of the row's agents first saw it,
  latest first. The default.
- **Oldest first** - the same time, earliest first.
- **Needs attention** - open rows that cannot land first, in the precedence the
  row colours use: conflicting or otherwise blocked, then behind its base, then
  checks running; then open rows whose mergeability is not known; then
  mergeable rows; then pending rows; then not-found rows; then closed rows;
  then merged rows.
- **Repository** - by `<owner>/<repo>` alphabetically, ignoring case, then by
  pull request number ascending.

Rows that the chosen order ranks equal SHALL fall back to newest first, so the
order is stable from one frame to the next. Sorting SHALL NOT reorder the
groups themselves.

A row whose state has not been fetched SHALL be sorted by what it has: its
repository and number are parsed from its URL, so the repository order does
not depend on state having arrived.

#### Scenario: Oldest first

- **WHEN** an agent opened pull request A on Monday and B on Tuesday, and the
  user chooses oldest first
- **THEN** A is listed above B in that agent's group

#### Scenario: Needs attention

- **WHEN** a group holds a merged row, a mergeable open row and a conflicting
  open row, and the user chooses needs attention
- **THEN** the conflicting row is first, the mergeable row second and the merged
  row last

#### Scenario: Repository order

- **WHEN** a group holds `acme/widget#9`, `acme/gadget#12` and `acme/widget#3`,
  and the user chooses repository
- **THEN** the order is `acme/gadget#12`, `acme/widget#3`, `acme/widget#9`

#### Scenario: Groups keep their order

- **WHEN** the user chooses any sort order
- **THEN** groups headed by several agents are still above single-agent groups

### Requirement: A search or filter that hides every row says so

When the workspace has recorded pull requests but the search and filters leave
no row to show, the view SHALL say that no pull request matches, and SHALL offer
a control that clears the search and every filter. It SHALL NOT show the message
for a workspace with no pull requests at all, which says the workspace has none.

Clearing SHALL empty the search field, deselect every status toggle and return
the agent picker to "All agents". It SHALL NOT change the sort order.

#### Scenario: Nothing matches

- **WHEN** the user searches for text no row contains
- **THEN** the view says no pull request matches and offers to clear the search
  and filters

#### Scenario: Clearing restores the list

- **WHEN** the user activates that control
- **THEN** the search is empty, no filter is selected, and every row is shown
  in the sort order the user chose

### Requirement: The user can act on the whole list

The toolbar SHALL offer a list actions menu holding:

- **Refresh now** - fetch the state of every listed pull request again, now,
  rather than when each would next age out. This includes rows hidden by the
  search or filters and rows previously found not to exist, and it asks again
  whether the tool is installed and authenticated. Rows SHALL keep showing the
  state they have until the new answer arrives, as "A recorded pull request's
  state is fetched and refreshed" requires of every refresh.
- **Copy URLs** - put the URLs of the rows currently shown on the clipboard, one
  per line, in the order they are shown.
- **Remove merged**, **Remove closed**, **Remove not found** - remove the rows
  currently shown that have that status.
- **Remove all** - remove every row currently shown.

The copy and remove actions SHALL act only on the rows the search and filters
leave shown, so a row the user cannot see is never removed. With no search and
no filter, Remove all removes every pull request in the workspace's list. Each
of these actions SHALL be disabled when it would act on no row.

Every removal SHALL ask for confirmation first. The confirmation SHALL state how
many pull requests it removes, SHALL say that nothing changes on GitHub, and,
while a search or filter is active, SHALL say that only the shown rows are
removed. Each removed row SHALL be removed exactly as "The user can remove a
recorded pull request" and "Removing a shared row removes it for every agent"
describe: for every agent it is attributed to, persistently, and recorded again
if an agent's output carries its URL again.

A removal SHALL write the recorded pull requests once, not once per row.

#### Scenario: Refreshing a not-found row

- **WHEN** a row has been found not to exist and the user chooses Refresh now
- **THEN** its state is fetched again without restarting Knot

#### Scenario: Refreshing does not blank rows

- **WHEN** the user chooses Refresh now while rows show their state
- **THEN** each row keeps its state until its new answer arrives

#### Scenario: Copying the shown URLs

- **WHEN** the view is filtered to open and shows two rows, and the user chooses
  Copy URLs
- **THEN** the clipboard holds those two URLs, one per line, and no others

#### Scenario: Removing the merged rows

- **WHEN** the list holds two merged rows and one open row, and the user chooses
  Remove merged and confirms
- **THEN** the two merged rows are gone, the open row remains, and nothing on
  GitHub changes

#### Scenario: Remove all respects the search

- **WHEN** the user searches for `acme/widget`, three of five rows match, and
  the user chooses Remove all
- **THEN** the confirmation says three pull requests will be removed from the
  shown rows, and after confirming the other two remain

#### Scenario: Clearing the whole list

- **WHEN** no search or filter is active and the user chooses Remove all and
  confirms
- **THEN** the view says the workspace has no pull requests, and the launcher
  row shows none

#### Scenario: Declining the confirmation

- **WHEN** the user chooses Remove closed and cancels the confirmation
- **THEN** no row is removed

#### Scenario: An action with nothing to act on

- **WHEN** no shown row is merged
- **THEN** Remove merged is disabled

#### Scenario: A bulk removal survives a restart

- **WHEN** the user removes every closed row and restarts Knot
- **THEN** none of them is listed

### Requirement: A row offers its actions in a context menu

A secondary click on a row SHALL open a menu holding Open in browser, Copy URL
and Remove. Open in browser SHALL behave as a click on the row does. Copy URL
SHALL put the row's URL on the clipboard. Remove SHALL behave as the row's
remove control does, confirmation included.

A secondary click SHALL NOT itself open the pull request in the browser.

#### Scenario: Copying one URL

- **WHEN** the user secondary-clicks a row and chooses Copy URL
- **THEN** the clipboard holds that row's URL and no browser is opened

#### Scenario: Removing from the menu

- **WHEN** the user secondary-clicks a row and chooses Remove
- **THEN** the same confirmation as the row's remove control is shown
