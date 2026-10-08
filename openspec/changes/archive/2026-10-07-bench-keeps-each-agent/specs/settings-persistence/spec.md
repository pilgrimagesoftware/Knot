# Spec Delta

## MODIFIED Requirements

### Requirement: Bench templates

The system SHALL store bench agents (reusable templates: id, name, avatar,
folder, agent type, optional shell command, optional persona id, description,
capabilities, cost tier, optional startup prompt). Adding a bench entry SHALL
replace an existing entry only when it has both the same name and the same
folder - the same agent saved again. An entry for a different agent in the
same folder SHALL be added beside it, not replace it: several agents commonly
work in one repository folder (#500).

Description, capabilities and cost tier are the registry fields defined by
`agent-registry`, and they SHALL round-trip: saving an agent to the bench
records them, and deploying the entry restores them onto the created agent. A
stored entry written before these fields existed SHALL load with an empty
description, no capability tags, and cost tier `medium`.

The startup prompt SHALL round-trip the same way, in whichever form it had: a
library reference stays a reference and custom text stays custom text. A
stored entry written before the startup prompt existed SHALL load with no
startup prompt.

#### Scenario: Same-folder bench entry replaced

- **WHEN** a bench entry is added for a folder that already has an entry with
  the same name - the same agent saved again
- **THEN** the old entry is removed and only the new one remains

#### Scenario: Another agent from the same folder is kept

- **WHEN** the bench holds `Reviewer` for `/repo` and `Tester` for `/repo` is
  saved
- **THEN** the bench holds both `Reviewer` and `Tester`

#### Scenario: Registry fields round-trip through the bench

- **WHEN** an agent tagged `testing` at cost tier `low` is saved to the bench
  and the settings file is reloaded
- **THEN** the stored entry still carries the tag and cost tier `low`

#### Scenario: Legacy bench entry loads with defaults

- **WHEN** a stored bench entry predates the registry fields
- **THEN** it loads with an empty description, no capability tags, and cost
  tier `medium`

#### Scenario: A library reference survives the bench

- **WHEN** an agent whose startup prompt references library prompt P is
  saved to the bench and the bench document is reloaded
- **THEN** the stored entry's startup prompt is still a reference to P, not a
  copy of P's text
