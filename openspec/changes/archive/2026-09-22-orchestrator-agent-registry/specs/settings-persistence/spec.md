# Spec Delta

## MODIFIED Requirements

### Requirement: Bench templates

The system SHALL store bench agents (reusable templates: id, name, avatar,
folder, agent type, optional shell command, optional persona id, description,
capabilities, cost tier). Adding a bench entry SHALL replace any existing
entry with the same folder.

Description, capabilities and cost tier are the registry fields defined by
`agent-registry`, and they SHALL round-trip: saving an agent to the bench
records them, and deploying the entry restores them onto the created agent. A
stored entry written before these fields existed SHALL load with an empty
description, no capability tags, and cost tier `medium`.

#### Scenario: Same-folder bench entry replaced

- **WHEN** a bench entry is added for a folder that already has one
- **THEN** the old entry is removed and only the new one remains

#### Scenario: Registry fields round-trip through the bench

- **WHEN** an agent tagged `testing` at cost tier `low` is saved to the bench
  and the settings file is reloaded
- **THEN** the stored entry still carries the tag and cost tier `low`

#### Scenario: Legacy bench entry loads with defaults

- **WHEN** a stored bench entry predates the registry fields
- **THEN** it loads with an empty description, no capability tags, and cost
  tier `medium`
