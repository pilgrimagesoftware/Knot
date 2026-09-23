# Spec Delta

## Purpose

Defines how Knot notices that an agent has opened a pull request, what it
records about it, how that record's state is kept current, and the view that
lists a workspace's pull requests and opens one in a browser.

## ADDED Requirements

### Requirement: A pull request URL in an agent's output is recorded

Knot SHALL watch each agent's output for GitHub pull request URLs and record
every distinct one it sees, attributed to the agent whose output carried it and
to that agent's workspace at the time.

A pull request URL is one of the form
`https://<host>/<owner>/<repo>/pull/<number>`, where `<host>` is `github.com`
or a GitHub Enterprise host, and `<number>` is a positive integer. A trailing
path or query - `/files`, `#issuecomment-…` - SHALL be stripped, so the same
pull request linked twice in different forms is recorded once. A URL that does
not match SHALL NOT be recorded; Knot records pull requests, not links.

Both agent kinds SHALL be covered. For an agent whose conversation runs over
the panel protocol, the text of its tool calls and their results is the output
watched. For an agent running in a terminal, the terminal's rendered text is.
An agent that opens a pull request in either mode is the same event to the user,
so it SHALL be the same record.

Recording SHALL be idempotent per agent: seeing the same pull request URL again
- because it scrolled past twice, or because a tool call was re-rendered -
SHALL NOT produce a second record, and SHALL NOT change the first-seen time.

The same pull request seen from two different agents SHALL be recorded once per
agent, because who opened it is part of what is recorded.

Watching output SHALL NOT delay or alter what the agent displays. Output is
read as it is already being processed, not by re-reading history.

#### Scenario: A panel agent opens a pull request

- **WHEN** a panel-mode agent's tool call output contains
  `https://github.com/acme/widget/pull/42`
- **THEN** that pull request is recorded against that agent and workspace,
  with the time it was first seen

#### Scenario: A terminal agent opens a pull request

- **WHEN** a terminal-mode agent runs a command that prints
  `https://github.com/acme/widget/pull/42`
- **THEN** the same record is produced as for a panel-mode agent

#### Scenario: The same URL twice

- **WHEN** an agent's output contains the same pull request URL three times
- **THEN** one record exists, with the first-seen time of the first occurrence

#### Scenario: A decorated URL is the same pull request

- **WHEN** an agent's output contains
  `https://github.com/acme/widget/pull/42/files` after
  `https://github.com/acme/widget/pull/42`
- **THEN** one record exists

#### Scenario: Two agents, one pull request

- **WHEN** two agents in a workspace each print the same pull request URL
- **THEN** two records exist, one per agent

#### Scenario: Not a pull request

- **WHEN** an agent's output contains `https://github.com/acme/widget/issues/42`
  or `https://github.com/acme/widget`
- **THEN** nothing is recorded

### Requirement: Recorded pull requests outlive the session

A recorded pull request SHALL persist across a restart of Knot, so a workspace's
list is the set of pull requests its agents have opened, not the set opened
since launch.

A record SHALL carry the pull request's URL, the agent id and workspace id it
was recorded against, and the time it was first seen. It SHALL NOT carry
fetched state - title, number, status - which is refreshed rather than
remembered, so a restart never shows a stale status as though it were current.

Removing an agent or a workspace SHALL remove the pull requests recorded
against it. Knot is showing what its agents produced; a record with no agent
has nothing to show it under.

The Swift reference records nothing of the kind.

#### Scenario: Records survive a restart

- **WHEN** an agent has opened two pull requests and Knot is quit and relaunched
- **THEN** both are still listed for that workspace

#### Scenario: A restart shows no stale status

- **WHEN** Knot is relaunched and the list is opened before state has been
  fetched
- **THEN** each row shows its URL with its state pending, not the state it had
  before the restart

#### Scenario: Removing an agent removes its records

- **WHEN** the user removes an agent that had opened a pull request
- **THEN** that pull request no longer appears in the workspace's list

### Requirement: A recorded pull request's state is fetched and refreshed

Knot SHALL fetch each recorded pull request's current state and show it: its
number, its title, whether it is draft, open, merged or closed, and the summary
result of its checks.

State SHALL be refreshed on a cadence while the list is being shown, and SHALL
NOT be fetched while drawing it. A row SHALL show the last state fetched until a
newer one arrives, rather than blanking on each refresh.

Fetching SHALL NOT block the interface. A list with twenty pull requests in it
SHALL remain scrollable and clickable while their states are being fetched.

#### Scenario: State appears

- **WHEN** the list is opened with a recorded pull request that is open with
  passing checks
- **THEN** that row shows its number, title, open state and passing checks

#### Scenario: State follows the pull request

- **WHEN** a listed pull request is merged on GitHub and the list is left open
- **THEN** the row shows it merged without the user reopening the view

#### Scenario: Refreshing does not blank the row

- **WHEN** a refresh is in flight for a row that already has state
- **THEN** the row keeps showing the state it has

### Requirement: Knot degrades rather than fails when state cannot be fetched

When the tool Knot uses to read pull request state is unavailable, not
authenticated, or fails for a given pull request, the recorded pull requests
SHALL still be listed with their URLs and SHALL still open in a browser. Only
the state is absent.

The reason SHALL be stated once for the view - not repeated on every row - and
SHALL distinguish "the tool is not installed", "it is not authenticated" and "a
request failed", because the three have different fixes.

A failure to fetch SHALL NOT remove a record, and SHALL NOT stop later refreshes
from being attempted.

#### Scenario: The tool is not installed

- **WHEN** the user opens the list on a machine where the tool is absent
- **THEN** every recorded pull request is listed with its URL and is clickable,
  and one message states that the tool is not installed

#### Scenario: Not authenticated

- **WHEN** the tool is installed but not authenticated
- **THEN** the list behaves the same and the message says so instead

#### Scenario: One pull request fails

- **WHEN** state is fetched for three pull requests and one request fails
- **THEN** the other two show their state, the failed one shows its URL without
  state, and no record is removed

#### Scenario: Recovery

- **WHEN** the user authenticates the tool and the next refresh runs
- **THEN** state appears without the user restarting Knot

### Requirement: The workspace window lists its pull requests

The workspace window SHALL offer a Pull Requests view listing the pull requests
recorded against that workspace's agents, reached from a launcher row in the
agent sidebar shaped like the Dashboard row described in `dashboard`, and shown
in place of the window's content the way the dashboard view is.

The list SHALL group rows by the agent that opened them and order them newest
first within each group, so the most recent work is at the top.

A workspace with no recorded pull requests SHALL say so rather than showing an
empty list.

The launcher row SHALL show how many of the workspace's recorded pull requests
are open, how many are merged and how many are closed, so the state of the
session's output is readable without opening the view. A draft pull request
SHALL count as open, matching the forge's own vocabulary, in which draft is a
property of an open pull request rather than a fourth state.

A state Knot has not fetched SHALL NOT be guessed at. Where no state is known -
before the view has been shown for the first time, or where the tool that reads
state is unavailable - the row SHALL show the total count instead. Where some
are known and others are not, the row SHALL show the states it knows and count
the rest as pending, rather than folding them into any state.

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

### Requirement: A listed pull request opens in the browser

Clicking a row in the Pull Requests view SHALL open that pull request's URL in
the user's default browser. Knot SHALL NOT open it in an embedded view.

If the URL cannot be opened, the failure SHALL be reported without removing the
record or closing the view.

#### Scenario: Opening a pull request

- **WHEN** the user clicks a row
- **THEN** the default browser opens that pull request's page

#### Scenario: The row is clickable without state

- **WHEN** state could not be fetched and the user clicks the row
- **THEN** the browser opens that pull request's page

### Requirement: The user can remove a recorded pull request

The Pull Requests view SHALL let the user remove a row from the list. Removal
SHALL affect only Knot's record: nothing is closed, deleted or changed on
GitHub.

A removed pull request SHALL stay removed across a restart, and SHALL be
recorded again if the agent's output carries its URL again - Knot is recording
what it sees, and it has seen it again.

#### Scenario: Removing a row

- **WHEN** the user removes a row from the list
- **THEN** it no longer appears, and the pull request on GitHub is unchanged

#### Scenario: A removal persists

- **WHEN** the user removes a row and restarts Knot
- **THEN** it is still absent
