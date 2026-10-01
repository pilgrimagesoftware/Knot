# Spec Delta

## ADDED Requirements

### Requirement: The workspace list scrolls

The list of workspaces in the Workspaces window SHALL be vertically
scrollable when its rows do not fit within the window's content area, so
every workspace remains reachable. A row that exists but is lower than the
visible area SHALL be reachable by scrolling the list, with no other action
required. When the rows do fit within the content area, the list SHALL show
all of them without scrolling.

This is the intended behavior of the port. The current Swift app always
wraps its workspace list in a `ScrollView`, scrolling affordance and all,
even when the contents are short; the port instead treats scrolling as a
capability that exists only once the rows overflow.

#### Scenario: Rows taller than the window scroll

- **WHEN** the Workspaces window is open and its workspace rows are taller
  than the window's content area
- **THEN** the user can scroll the list vertically, and every row below the
  fold becomes reachable by scrolling

#### Scenario: Rows that fit need no scroll

- **WHEN** the workspace rows fit within the window's content area
- **THEN** every workspace row is visible without scrolling

#### Scenario: The last workspace is reachable

- **WHEN** the window lists more workspaces than fit in its content area and
  the user scrolls the list to its end
- **THEN** the last workspace's row is visible