# Spec Delta

## MODIFIED Requirements

### Requirement: The user can remove a recorded pull request

The Pull Requests view SHALL let the user remove a row from the list. Removal
SHALL affect only Knot's record: nothing is closed, deleted or changed on
GitHub.

A removed pull request SHALL stay removed across a restart, and SHALL be
recorded again if the agent's output carries its URL again - Knot is recording
what it sees, and it has seen it again.

Activating the remove control SHALL perform the removal and nothing else. It
sits inside a row that opens the pull request when clicked, and it SHALL NOT
also count as a click on that row: the user asking to forget a record is not
asking to visit it. This holds for any control later placed inside one of these
rows — a control within a row acts for itself alone.

A click on the row away from any such control SHALL still open the pull
request, as "A listed pull request opens in the browser" requires.

#### Scenario: Removing a row

- **WHEN** the user removes a row from the list
- **THEN** it no longer appears, and the pull request on GitHub is unchanged

#### Scenario: A removal persists

- **WHEN** the user removes a row and restarts Knot
- **THEN** it is still absent

#### Scenario: Removing does not open

- **WHEN** the user activates a row's remove control
- **THEN** the removal is offered and no browser is opened for that pull
  request

#### Scenario: The row still opens beside the control

- **WHEN** the user clicks a row anywhere other than its remove control
- **THEN** the default browser opens that pull request's page
