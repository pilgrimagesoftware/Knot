## MODIFIED Requirements

### Requirement: A recorded pull request's state is fetched and refreshed

Knot SHALL fetch each recorded pull request's current state and show it: its
number, its title, whether it is draft, open, merged or closed, and, while it
is draft or open, the summary result of its checks. A merged or closed pull
request's row SHALL NOT show a checks result.

An open or draft pull request's row SHALL be coloured by whether it can land
and, if not, why: one colour when it is mergeable, one when it conflicts with
its base or is otherwise blocked, one when its branch is behind its base, and
one while its checks are running. Where more than one applies, a conflict
SHALL take precedence, then a draft or failing checks, then being behind, then
running checks. A row whose mergeability has not been computed SHALL NOT be
coloured as mergeable.

State SHALL be refreshed on a cadence while the list is being shown, and SHALL
NOT be fetched while drawing it. A row SHALL show the last state fetched until a
newer one arrives, rather than blanking on each refresh.

Fetching SHALL NOT block the interface. A list with twenty pull requests in it
SHALL remain scrollable and clickable while their states are being fetched.

#### Scenario: State appears

- **WHEN** the list is opened with a recorded pull request that is open with
  passing checks
- **THEN** that row shows its number, title, open state and passing checks

#### Scenario: A decided pull request shows no checks

- **WHEN** a listed pull request is merged or closed
- **THEN** its row shows its number, title and state, and no checks result

#### Scenario: An open row says why it cannot land

- **WHEN** a listed open pull request conflicts with its base, is behind it,
  or has checks running
- **THEN** its row wears the colour for that reason, distinct from the colour
  of a mergeable one

#### Scenario: State follows the pull request

- **WHEN** a listed pull request is merged on GitHub and the list is left open
- **THEN** the row shows it merged without the user reopening the view

#### Scenario: Refreshing does not blank the row

- **WHEN** a refresh is in flight for a row that already has state
- **THEN** the row keeps showing the state it has
