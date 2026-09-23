# terminal-rendering Specification

## Purpose
Gives each agent's PTY session a real on-screen terminal surface inside its
workspace window, replacing the static placeholder with a live, parsed
terminal grid the user can read and type into.

## Requirements

### Requirement: Selected agent shows a live terminal surface

The workspace window SHALL render the selected agent's terminal contents in
its content pane, in place of the current static placeholder. A non-shell
agent's surface SHALL reflect the same PTY output already driving its
status/title tracking (`activity-detection`, `agent-hooks`).

#### Scenario: Selecting an agent shows its terminal

- **WHEN** the user selects an agent in the workspace sidebar
- **THEN** that agent's terminal surface becomes visible in the content pane
- **AND** any other agent's surface is hidden or detached

#### Scenario: No agent selected

- **WHEN** no agent is selected in the workspace
- **THEN** the content pane shows no terminal surface (falls back to the
  existing "choose an agent" state)

### Requirement: Surface lifecycle tracks agent lifecycle

A terminal surface SHALL be created no later than when its agent's PTY
session starts, and SHALL be torn down when the agent is removed or its
session restarts (matching `agent-lifecycle`'s restart semantics - a
restarted agent gets a fresh surface, not a reused one).

#### Scenario: Removing an agent tears down its surface

- **WHEN** an agent is removed from a workspace
- **THEN** its terminal surface and underlying resources are released
- **AND** no further rendering work occurs for that surface

#### Scenario: Restarting an agent gets a fresh surface

- **WHEN** an agent is restarted (`agent-lifecycle`'s restart operation)
- **THEN** its old surface is torn down and a new surface is created for the
  new PTY session

### Requirement: Surface resizes with its container

The rendered surface SHALL track the size of the content pane it occupies,
including window resizes, reflowing its row/column grid accordingly and
resizing the underlying PTY to match.

#### Scenario: Window resize reflows the terminal grid

- **WHEN** the workspace window is resized
- **THEN** the visible terminal surface's row/column grid reflows to match
  its content pane's new size
- **AND** the underlying PTY is resized to match (so the running program
  sees the new dimensions)

### Requirement: Drawing a terminal frame does not enumerate installed fonts

Drawing or sizing the terminal surface SHALL NOT enumerate the fonts
installed on the machine. The family the surface draws in SHALL be resolved
when the configured terminal font name is first seen and reused for every
subsequent frame that asks for the same name, so per-frame cost is
independent of how many fonts are installed.

Resolution behaviour is unchanged: a configured name the text system cannot
resolve SHALL still fall back to the embedded default monospace family
rather than to the proportional UI font.

The Swift app does not have this cost - it hands the family name to a native
terminal surface and never asks the text system which families exist.

#### Scenario: Repeated frames resolve the same family once

- **WHEN** the terminal surface draws many frames with the terminal font
  setting unchanged
- **THEN** the installed font families are enumerated at most once across
  those frames

#### Scenario: Changing the terminal font takes effect

- **WHEN** the user changes the terminal font name in settings
- **THEN** the next frame draws in the newly configured family, resolving it
  against the installed families

#### Scenario: An uninstalled family still falls back

- **WHEN** the configured terminal font name is not installed
- **THEN** the surface draws in the embedded default monospace family
- **AND** subsequent frames reuse that fallback without enumerating again
