## ADDED Requirements

### Requirement: A finished tool call collapses to its header

A tool call that has completed successfully SHALL render collapsed: its
header - icon, title and status - stays visible and its output is hidden.

A tool call that is still running SHALL stay expanded, since its output is
what the user is waiting on.

A tool call that failed SHALL stay expanded. Failure output is the reason the
user is reading the conversation at all, and hiding it behind a control makes
the one card that matters the one card they have to open.

Collapsing SHALL NOT discard anything: the content is hidden, not dropped,
and opening the card again shows exactly what was there.

#### Scenario: A call collapses when it succeeds

- **WHEN** a tool call the panel is showing expanded reaches `completed`
- **THEN** its output is hidden and its header remains, naming the tool and
  its status

#### Scenario: A running call stays open

- **WHEN** a tool call is `pending` or `in_progress`
- **THEN** it renders expanded

#### Scenario: A failed call stays open

- **WHEN** a tool call reaches `failed`
- **THEN** it renders expanded, with its output visible

#### Scenario: Reopening shows the same content

- **WHEN** the user opens a collapsed call
- **THEN** the output shown is the content that call finished with

### Requirement: Every tool call can be opened and closed

Each tool call card SHALL carry a control that toggles it between collapsed
and expanded, and that shows which of the two it currently is.

The control SHALL be present whatever the call's status - a running call can
be collapsed, a failed one can be closed - so the automatic behaviour above
is a default rather than a rule the user cannot escape.

#### Scenario: Closing a call the panel opened

- **WHEN** the user activates the control on an expanded call
- **THEN** that call collapses to its header

#### Scenario: Opening a call the panel closed

- **WHEN** the user activates the control on a collapsed call
- **THEN** that call expands and its output is shown

#### Scenario: A running call can be closed

- **WHEN** the user activates the control on an `in_progress` call
- **THEN** it collapses, and goes on streaming its output out of sight

### Requirement: The user's choice outlives the automatic one

Once the user has opened or closed a particular tool call, that choice SHALL
hold for that call for the rest of the session, including across the call
finishing. The automatic collapse SHALL apply only to a call the user has not
touched.

#### Scenario: A call opened while running stays open when it finishes

- **WHEN** the user opens a running call and it then reaches `completed`
- **THEN** it stays expanded

#### Scenario: A call closed while running stays closed when it finishes

- **WHEN** the user closes a running call and it then reaches `completed`
- **THEN** it stays collapsed

#### Scenario: An untouched call follows the default

- **WHEN** a call the user has never toggled reaches `completed`
- **THEN** it collapses

### Requirement: Collapse state is per call and not persisted

The open/closed state SHALL be tracked per tool call, so opening one leaves
the others as they were, and SHALL NOT be persisted: a reloaded conversation
starts from the automatic behaviour again.

#### Scenario: Opening one call leaves the others alone

- **WHEN** the user opens one of several collapsed calls
- **THEN** only that one expands

#### Scenario: A reloaded conversation starts fresh

- **WHEN** a session is restarted or its conversation reloaded
- **THEN** every finished call renders collapsed again, whatever the user had
  opened before

### Requirement: Collapsing does not move what the user is reading

A call collapsing on completion SHALL NOT scroll the conversation away from
what the user is looking at. While auto-scroll is following a response, the
panel SHALL stay at the end of the conversation; while it is not, the content
the user is reading SHALL stay where it is.

#### Scenario: A call finishes while the user reads earlier output

- **WHEN** the user has scrolled back to read earlier output and a call
  further down completes and collapses
- **THEN** what the user is reading stays in place

#### Scenario: A call finishes while auto-scroll is following

- **WHEN** auto-scroll is following a streaming response and a call above it
  collapses
- **THEN** the panel stays at the end of the conversation
