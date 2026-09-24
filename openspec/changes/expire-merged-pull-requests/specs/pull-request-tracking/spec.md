# Spec Delta

## ADDED Requirements

### Requirement: A merged pull request stops being listed once it is old

A recorded pull request that has been merged for longer than a retention window
of 24 hours SHALL be dropped from the workspace's list. The list is what the
workspace still has in flight, not a ledger of everything its agents ever
opened; work that merged yesterday is finished, and leaving it in the list
crowds out the pull requests that are not.

Expiry SHALL be measured against the merge time the forge reports, not against
when Knot first saw the URL. When an agent first printed a URL says nothing
about when that pull request merged, so a record first seen an hour ago may
already be well past the window and one first seen a month ago may have merged
a minute ago.

A record SHALL be dropped only on positively fetched evidence: its state is
merged and its merge time is known and older than the window. A record whose
state has not been fetched, whose fetch failed, or whose merge time the forge
did not report SHALL be kept. Knot removes what it has observed, and declines
to guess when it has not.

Only merged pull requests SHALL expire. An open, draft or closed pull request
SHALL remain listed however old it is - a closed pull request is often
abandoned work the user still wants to see.

Expiry SHALL affect only Knot's record. Nothing is closed, deleted or changed
on the forge, exactly as for a pull request the user removes by hand.

An expired record SHALL stay expired across a restart, and SHALL be recorded
again if an agent's output carries its URL again - the same rule that governs a
record the user removed. Once recorded again it SHALL expire again on the next
fetch that resolves it, because it is still an old merged pull request.

Expiry SHALL be reflected in the workspace's list and in the launcher row's
counts without the user reopening the view or the window.

The Swift reference has no pull request list, so it has nothing of the kind.

#### Scenario: A pull request merged yesterday

- **WHEN** a recorded pull request's fetched state is merged and the forge
  reports it merged 30 hours ago
- **THEN** the record is dropped from the workspace's list

#### Scenario: A pull request merged an hour ago

- **WHEN** a recorded pull request's fetched state is merged and the forge
  reports it merged one hour ago
- **THEN** the record stays listed

#### Scenario: An old merge seen for the first time

- **WHEN** an agent prints the URL of a pull request that merged a month ago
  and the first fetch resolves it
- **THEN** the record is dropped, because the retention window is measured from
  the merge rather than from the sighting

#### Scenario: A closed pull request does not expire

- **WHEN** a recorded pull request's fetched state is closed and it was closed
  a week ago
- **THEN** the record stays listed

#### Scenario: No state, no expiry

- **WHEN** a record's state has not been fetched, or its fetch failed
- **THEN** the record stays listed with its URL, and is reconsidered on a later
  refresh

#### Scenario: Merged with no merge time reported

- **WHEN** the forge reports a pull request as merged but reports no merge time
- **THEN** the record stays listed

#### Scenario: Expiry survives a restart

- **WHEN** a record expires and Knot is quit and relaunched
- **THEN** it is not listed

#### Scenario: The same pull request printed again

- **WHEN** an agent's output carries the URL of a pull request that previously
  expired
- **THEN** it is recorded again and appears in the list, and is dropped again on
  the next fetch that resolves it as merged and old

#### Scenario: The launcher row follows

- **WHEN** a listed merged pull request passes the retention window while the
  workspace window is open
- **THEN** the launcher row's merged count drops by one without the user
  reopening the window

## MODIFIED Requirements

### Requirement: Recorded pull requests outlive the session

A recorded pull request SHALL persist across a restart of Knot, so a workspace's
list is the set of pull requests its agents have opened, not the set opened
since launch. It SHALL persist until the user removes it, until the agent or
workspace it was recorded against is removed, or until it expires under the
retention window for merged pull requests.

A record SHALL carry the pull request's URL, the agent id and workspace id it
was recorded against, and the time it was first seen. It SHALL NOT carry
fetched state - title, number, status, merge time - which is refreshed rather
than remembered, so a restart never shows a stale status as though it were
current. Expiry SHALL therefore be decided from freshly fetched state after a
restart, not from anything the record remembers.

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

#### Scenario: A record kept after a restart is re-judged, not remembered

- **WHEN** Knot is relaunched with a record whose pull request merged two days
  ago
- **THEN** it is listed until its state is fetched, and dropped once that fetch
  reports it merged and past the window

### Requirement: A recorded pull request's state is fetched and refreshed

Knot SHALL fetch each recorded pull request's current state and show it: its
number, its title, whether it is draft, open, merged or closed, and the summary
result of its checks. For a merged pull request the fetched state SHALL also
carry the time it was merged, which is what the retention window is measured
against. The merge time is not shown in a row; it is fetched because expiry
needs it.

The merge time SHALL be treated as absent rather than guessed at when the forge
does not report it, so that a forge or tool version that omits it costs the user
nothing beyond records that do not expire.

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

#### Scenario: A merged pull request's state carries its merge time

- **WHEN** state is fetched for a pull request the forge reports as merged
- **THEN** the fetched state carries the merge time the forge reported

#### Scenario: An unmerged pull request has no merge time

- **WHEN** state is fetched for a pull request that is open, draft or closed
  without merging
- **THEN** the fetched state carries no merge time
