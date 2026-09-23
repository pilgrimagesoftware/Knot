# Spec Delta

## MODIFIED Requirements

### Requirement: Command Center window

A separate, global dashboard window ("Command Center") SHALL show every
attached workspace's agents, reusing the same agent card grid as the
workspace-scoped dashboard.

It SHALL be reachable from the Window menu and its key equivalent, per
`app-menu`, as well as from the workspace manager's toolbar. Only one
Command Center window SHALL exist at a time; a second request to open it
activates the one already open, per `window-lifecycle`.

Its content SHALL scroll vertically when the workspace sections and their
cards are taller than the window's content area, so every workspace and every
card remains reachable at any window size. Cards SHALL NOT be shrunk or
dropped to make the grid fit. The card grid is the Command Center's whole
purpose, and a user with several workspaces reaches the bottom of it in the
window's default size.

#### Scenario: Command Center shows all workspaces

- **WHEN** the Command Center window is open
- **THEN** every attached workspace appears as its own section in the
  grid, not just one workspace

#### Scenario: Card in Command Center opens the workspace window

- **WHEN** an agent's card is clicked in the Command Center
- **THEN** that agent's workspace window opens (or focuses, if already
  open) showing its terminal view with that agent selected

#### Scenario: The grid scrolls when it overflows

- **WHEN** the Command Center holds more workspace sections and cards than
  fit its content area
- **THEN** the content scrolls vertically, and the last card of the last
  workspace can be scrolled to and clicked

#### Scenario: A short grid does not scroll

- **WHEN** the Command Center's content is shorter than its content area
- **THEN** the content does not scroll and no scrollbar occupies space
  the cards would otherwise use

#### Scenario: Resizing reveals the rest

- **WHEN** the user shrinks the Command Center window until its content
  overflows
- **THEN** the content becomes scrollable rather than clipped
