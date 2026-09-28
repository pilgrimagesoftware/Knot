# Spec Delta

## MODIFIED Requirements

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

The search field SHALL look like a field while it does not have focus: it
SHALL be outlined by a border in the window's separator color, distinguishable
from the field's fill and from the toolbar behind it, in light and dark
appearance. While it has focus it SHALL show the focus border every focused
control shows. This styling belongs to the search field alone; other text
fields keep their own appearance.

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

#### Scenario: The unfocused search field has a border

- **WHEN** the Pull Requests view is shown on macOS, in light or in dark
  appearance, and the search field does not have focus
- **THEN** the search field is outlined by a separator-colored border that
  stands out from its fill and the toolbar

#### Scenario: The focused search field shows the focus border

- **WHEN** the user clicks into the search field
- **THEN** its border changes to the focus border, as any focused control's
  does

