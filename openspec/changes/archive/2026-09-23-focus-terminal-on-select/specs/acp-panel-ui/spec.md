# Spec Delta

## MODIFIED Requirements

### Requirement: Selecting a Panel-mode agent focuses its prompt input

When a Panel-mode agent becomes a workspace window's selected agent and its
conversation is what the content area shows, that agent's prompt input SHALL
receive keyboard focus. The user SHALL be able to select an agent and begin
typing a message with no intervening click.

This SHALL hold however the selection was made: clicking the agent's sidebar
row, clicking its card in the window's overview, creating the agent, or the
window opening on a restored selection. This diverges from the Swift reference,
which leaves focus wherever it was.

Focus SHALL be taken once per selection, not held. Once the user moves focus
elsewhere in the window, it SHALL stay where they put it until the selection
changes again — a window SHALL NOT pull focus back into the composer while the
user is working somewhere else in it.

Focus SHALL NOT be taken when the composer is not what the content area shows.
That covers the window showing its dashboard rather than an agent, an open
markdown or diagram pane holding the content area ahead of the conversation,
and a deactivated agent whose pane shows the stopped placeholder. In each of
those cases focus SHALL be left where it is. A Terminal-mode agent focuses its
terminal surface instead, under `terminal-input`'s "Selecting a Terminal-mode
agent focuses its terminal surface"; no composer is focused for it.

Focus SHALL NOT be taken from a modal dialog while one is open.

Taking focus SHALL NOT raise, activate or reorder any window. It places focus
within a window; which window the system considers frontmost is unaffected.

Each agent's composer keeps its own contents, so returning to an agent SHALL
focus that agent's composer with the text the user last left in it, caret
placement included where the composer already preserves it.

#### Scenario: Selecting an agent and typing

- **WHEN** the user clicks a Panel-mode agent's sidebar row and types
- **THEN** the typed text goes into that agent's prompt input, with no click on
  the composer

#### Scenario: A window opens on a restored selection

- **WHEN** a workspace window opens with a Panel-mode agent as its restored
  selection and its conversation on screen
- **THEN** that agent's prompt input has keyboard focus

#### Scenario: A newly created agent is ready to be prompted

- **WHEN** the user creates a Panel-mode agent and the window selects it
- **THEN** its prompt input has keyboard focus

#### Scenario: Focus is not pulled back

- **WHEN** the user selects a Panel-mode agent, then clicks a control elsewhere
  in the window, and the window redraws several times
- **THEN** focus stays on the control the user clicked

#### Scenario: Returning to an agent refocuses its own composer

- **WHEN** the user types a partial message to one agent, selects a second
  agent, and then selects the first again
- **THEN** the first agent's prompt input has focus and still holds the partial
  message

#### Scenario: A Terminal-mode agent does not move focus

- **WHEN** the user selects an agent that runs in Terminal mode
- **THEN** no prompt input is focused
- **AND** that agent's terminal surface takes focus instead, per
  `terminal-input`'s "Selecting a Terminal-mode agent focuses its terminal
  surface"

#### Scenario: The dashboard is showing

- **WHEN** the window is showing its dashboard rather than an agent's
  conversation
- **THEN** no prompt input is focused

#### Scenario: A markdown pane holds the content area

- **WHEN** the user selects a Panel-mode agent that has a markdown file open, so
  the markdown pane takes the content area
- **THEN** focus is left where it was

#### Scenario: A deactivated agent is selected

- **WHEN** the user selects a Panel-mode agent that is deactivated, so its pane
  shows the stopped placeholder
- **THEN** focus is left where it was

#### Scenario: A dialog keeps focus

- **WHEN** a modal dialog is open and the window's selection changes beneath it
- **THEN** the dialog keeps focus
