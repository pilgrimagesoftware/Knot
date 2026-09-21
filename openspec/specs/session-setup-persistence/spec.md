# session-setup-persistence Specification

## Purpose
Preserves the setup choices that define an agent session - model, permission
mode and reasoning effort - so repeated prompts use what the user selected
rather than whatever the adapter defaults to. These axes are ACP Session
Config Options declared by the adapter at runtime, not a vocabulary Knot
defines, so the persisted shape is the adapter's own option id paired with
the selected value. The Rust port MAY use any backing store; the persisted
shapes and behaviors are the contract.

## Requirements

### Requirement: Session setup persists per agent

The system SHALL persist the selected model, permission mode, and reasoning
effort for each agent session and SHALL restore those values when the session
is reopened.

The setup SHALL be stored per agent, not globally: agents may run different
adapters, which declare different options and different legal values for
them.

The setup SHALL persist independently of `restore-conversation-on-launch`.
That preference governs conversation content; a setup choice is a preference
about how the next turn runs and outlives any one conversation.

#### Scenario: Restore saved setup
- **WHEN** an agent session is reopened after model, permission mode, and
  effort were selected
- **THEN** the session controls show and use the previously selected values

### Requirement: Legacy sessions use safe defaults

The system SHALL load sessions saved before these fields existed without
error and SHALL use the existing default model, permission mode, and effort
when a value is absent.

An option the adapter no longer declares SHALL be skipped when the setup is
restored, rather than failing the session's connection - adapters add and
drop config options between versions.

#### Scenario: Load a legacy session
- **WHEN** a saved session has no persisted setup fields
- **THEN** it opens successfully with the configured defaults

### Requirement: Setup changes apply to later turns

Changing a persisted setup value SHALL apply to the next message and later
turns without changing an already-running turn.

#### Scenario: Change setup during a turn
- **WHEN** the user changes effort while a response is in progress
- **THEN** the current response continues unchanged and the next message uses
  the new effort
