# Spec Delta

## ADDED Requirements

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
again while that window is open. The answer is not persisted, so it is asked
again after a restart - which is also what recovers a repository that becomes
readable once the tool is authenticated for it.

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

#### Scenario: A not-found row stays the user's to remove

- **WHEN** a row has been found not to exist
- **THEN** it is still listed, still opens in a browser when clicked, and is
  removed only when the user removes it

#### Scenario: Other failures still retry

- **WHEN** a fetch times out or its response cannot be parsed
- **THEN** the row is not shown as not found, and a later refresh fetches it
  again

## MODIFIED Requirements

### Requirement: Knot degrades rather than fails when state cannot be fetched

When the tool Knot uses to read pull request state is unavailable, not
authenticated, or fails for a given pull request, the recorded pull requests
SHALL still be listed with their URLs and SHALL still open in a browser. Only
the state is absent.

The reason SHALL be stated once for the view - not repeated on every row - and
SHALL distinguish "the tool is not installed", "it is not authenticated" and "a
request failed", because the three have different fixes.

A failure to fetch SHALL NOT remove a record, and SHALL NOT stop later refreshes
from being attempted. A pull request the forge says does not exist is not a
failure to fetch: "A pull request the forge does not know is shown as not found"
governs it.

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
