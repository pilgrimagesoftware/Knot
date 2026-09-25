# Spec Delta

## MODIFIED Requirements

### Requirement: Selecting a Terminal-mode agent focuses its terminal surface

When a Terminal-mode agent becomes a workspace window's selected agent and its
terminal surface is what the content area shows, that surface SHALL receive
keyboard focus. The user SHALL be able to select the agent and begin typing
into it with no intervening click.

This SHALL hold however the selection was made: clicking the agent's sidebar
row, clicking its card in the window's overview, creating the agent, or the
window opening on a restored selection. This diverges from the Swift
reference, which leaves focus wherever it was.

Focus SHALL be taken on the frame the surface first appears on, not on the
frame the selection changes. A session that is still starting shows a
placeholder rather than a surface, and focus SHALL NOT be taken then; the
frame its grid first appears on is the one that takes it.

Focus SHALL be taken once per selection, not held. Once the user moves focus
elsewhere in the window, it SHALL stay where they put it until the selection
changes again.

Focus SHALL NOT be taken when the surface is not what the content area shows -
the window showing its dashboard or pull requests rather than an agent, or a
deactivated agent whose pane shows the stopped placeholder. Focus SHALL NOT be
taken from a modal dialog while one is open. Taking focus SHALL NOT raise,
activate or reorder any window.

An open artifact panel SHALL withhold focus only while it is expanded. Under
`artifact-panel` an expanded panel takes the whole content area and the terminal
surface is not on screen, which is the case this exception is for. An unexpanded
panel is a sibling of the content pane, so the terminal surface is on screen
beside a shown markdown file or diagram and SHALL be focused as it is for any
other selected Terminal-mode agent. Merely having an artifact open is not the
condition.

Expanding or collapsing the panel SHALL NOT itself take focus. The panel's
expanded state decides whether focus can be taken for a selection, not whether
a new selection has happened; a panel returning to its set width makes the
terminal surface visible again and SHALL leave focus wherever the user last put
it, under the once-per-selection rule above. The state read SHALL be the panel's
live expanded state, which the user's own toggle changes, and not the
`maximized` argument last recorded for the agent — the two differ from the
moment the user collapses a panel an agent maximized.

A Terminal-mode agent can hold an artifact by three paths, so this is not a
vacuous case: any agent may be switched to Terminal view under `acp-panel-ui`'s
"View-mode toggle", including one that runs an MCP client of its own; an agent
may call `display-markdown` or `view-mermaid` naming a different agent, which
neither tool restricts by view mode or agent type; and the user may show a file
for any agent from `agent-list-ui`'s "Markdown Files" menu, which is offered
whenever that agent has markdown history.

#### Scenario: Selecting a shell agent and typing

- **WHEN** the user clicks a Terminal-mode agent's sidebar row and types
- **THEN** the typed text reaches that agent's PTY, with no click on the pane

#### Scenario: A session that is still starting

- **WHEN** the user selects a Terminal-mode agent whose session has not yet
  produced a grid, so the pane shows the starting placeholder
- **THEN** focus is not taken on that frame
- **AND** the frame the grid first appears on takes it

#### Scenario: Focus is not pulled back

- **WHEN** the user selects a Terminal-mode agent, then clicks a control
  elsewhere in the window, and the window redraws several times
- **THEN** focus stays on the control the user clicked

#### Scenario: Switching between a panel agent and a shell agent

- **WHEN** the user selects a Panel-mode agent, then a Terminal-mode agent
  whose grid is showing
- **THEN** the terminal surface has keyboard focus

#### Scenario: A shell agent with a diagram open is still focused

- **WHEN** the user selects a Terminal-mode agent that has a diagram open in an
  unexpanded panel, beside its terminal surface
- **THEN** that agent's terminal surface has keyboard focus

#### Scenario: An expanded panel withholds focus

- **WHEN** the user selects a Terminal-mode agent whose artifact panel is
  expanded, so the terminal surface is not on screen
- **THEN** focus is left where it was

#### Scenario: Collapsing an expanded panel does not pull focus

- **WHEN** the user selects a Terminal-mode agent whose panel is expanded,
  clicks a control elsewhere in the window, and then collapses the panel
- **THEN** focus stays on the control the user clicked

#### Scenario: A deactivated agent does not move focus

- **WHEN** the user selects a Terminal-mode agent that is not activated, so
  its pane shows the stopped placeholder
- **THEN** focus is left where it was
