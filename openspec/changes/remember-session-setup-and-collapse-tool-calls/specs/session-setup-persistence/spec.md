# Spec Delta

## Purpose

Preserves the setup choices that define an agent session so repeated prompts use the user's selected model, permissions, and reasoning effort.

## ADDED Requirements

### Requirement: Session setup persists per agent

The system SHALL persist the selected model, permission mode, and reasoning effort for each agent session and SHALL restore those values when the session is reopened.

#### Scenario: Restore saved setup
- **WHEN** an agent session is reopened after model, permission mode, and effort were selected
- **THEN** the session controls show and use the previously selected values

### Requirement: Legacy sessions use safe defaults

The system SHALL load sessions saved before these fields existed without error and SHALL use the existing default model, permission mode, and effort when a value is absent.

#### Scenario: Load a legacy session
- **WHEN** a saved session has no persisted setup fields
- **THEN** it opens successfully with the configured defaults

### Requirement: Setup changes apply to later turns

Changing a persisted setup value SHALL apply to the next message and later turns without changing an already-running turn.

#### Scenario: Change setup during a turn
- **WHEN** the user changes effort while a response is in progress
- **THEN** the current response continues unchanged and the next message uses the new effort
