# Spec Delta

## MODIFIED Requirements

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

#### Scenario: A pull request several agents opened

- **WHEN** two agents in the workspace have both recorded the same pull request
- **THEN** the list shows it once, in a group headed by both agents, above the
  single-agent groups
- **AND** the launcher row counts it once

## ADDED Requirements

### Requirement: Removing a shared row removes it for every agent

Removing a row listed for several agents SHALL forget the record for every agent
it is attributed to, so one row on screen is one removal. The row SHALL NOT drop
back into a remaining agent's group: a row the user removed reappearing under a
different heading reads as a bug. Removal otherwise follows "The user can remove
a recorded pull request".

#### Scenario: Removing a shared row

- **WHEN** two agents have recorded the same pull request and the user removes
  its row
- **THEN** the row no longer appears under either agent, and the launcher row no
  longer counts it
