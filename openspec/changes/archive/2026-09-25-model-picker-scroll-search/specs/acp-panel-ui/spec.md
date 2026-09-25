# Spec Delta

## ADDED Requirements

### Requirement: Model dropdown scrolls

When the models a selected agent declares exceed the height available to
the model selector's dropdown, the dropdown SHALL be vertically scrollable,
so every declared model remains reachable by scrolling, with no other action
required. When the declared models fit within the available height, the
dropdown SHALL show all of them without scrolling.

The Swift reference has no equivalent dropdown to keep parity with: the
model axis is offered from the agent's own declared Session Config Options,
which the port introduces. This requirement is intended port behavior.

#### Scenario: More models than fit scroll

- **WHEN** the agent declares more models than the open model dropdown can
  display at once
- **THEN** the user can scroll the dropdown, and every model below the fold
  becomes selectable by scrolling to it

#### Scenario: Models that fit need no scroll

- **WHEN** the agent declares at most as many models as the dropdown can
  display at once
- **THEN** every declared model is visible in the open dropdown without
  scrolling

#### Scenario: The last declared model is reachable

- **WHEN** the agent declares more models than the dropdown can display and
  the user scrolls the list to its end
- **THEN** the last declared model is visible and selectable

### Requirement: Model dropdown searches

The model selector's dropdown SHALL provide a search field that filters the
listed models by case-insensitive substring match on the model's displayed
name, so a model can be found by typing part of its name. While the search
field is non-empty, only models matching it SHALL be listed; a model that
does not match SHALL NOT be offered. Clearing the field SHALL restore the
full list.

#### Scenario: Typing narrows the list

- **WHEN** the user types text into the model dropdown's search field
- **THEN** only models whose displayed name contains that text (case-
  insensitive) are listed

#### Scenario: Empty search shows everything

- **WHEN** the search field is empty
- **THEN** every declared model is listed, in the agent's declared order

#### Scenario: No match selects nothing

- **WHEN** no declared model matches the search text
- **THEN** the dropdown offers no model to select, and the currently
  selected model is left unchanged until the user empties or changes the
  search text