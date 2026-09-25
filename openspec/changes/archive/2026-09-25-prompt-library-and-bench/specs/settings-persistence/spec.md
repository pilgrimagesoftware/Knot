# Spec Delta

## MODIFIED Requirements

### Requirement: Durable agent field set

A persisted agent (`SavedAgent`) SHALL store exactly: id, name, avatar (a
non-empty string, default a robot emoji), folder, agent type (default
`claude`), created-by, is-companion, shell command, persona id, an optional
session id, and an optional startup prompt (see `prompt-library` - Startup
prompt forms). Loading SHALL reconstruct agents with all runtime fields at
defaults except session id, which SHALL be reconstructed from the persisted
value when present (see `agent-lifecycle` - Durable versus runtime fields for
how it is then used to resolve resume-session id).

Session id SHALL only ever be written to a non-`None` value when
`restore-conversation-on-launch` is enabled at persist time; whenever it is
disabled, saving an agent SHALL persist session id as `None` regardless of
the agent's runtime session id.

A stored agent written before the startup prompt existed SHALL load with no
startup prompt.

#### Scenario: Avatar default on save

- **WHEN** an agent with no avatar is saved
- **THEN** its persisted avatar is the default robot emoji

#### Scenario: Session id omitted by default

- **WHEN** `restore-conversation-on-launch` is disabled (the default) and an
  agent with an active runtime session id is saved
- **THEN** the persisted agent's session id is `None`

#### Scenario: Startup prompt round-trips

- **WHEN** an agent with a custom startup prompt is saved and the agents
  document is reloaded
- **THEN** the reloaded agent carries the same custom startup prompt

#### Scenario: Legacy agent loads without a startup prompt

- **WHEN** a stored agent predates the startup prompt field
- **THEN** it loads with no startup prompt

### Requirement: Bench templates

The system SHALL store bench agents (reusable templates: id, name, avatar,
folder, agent type, optional shell command, optional persona id, description,
capabilities, cost tier, optional startup prompt). Adding a bench entry SHALL
replace any existing entry with the same folder.

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

#### Scenario: A library reference survives the bench

- **WHEN** an agent whose startup prompt references library prompt P is
  saved to the bench and the bench document is reloaded
- **THEN** the stored entry's startup prompt is still a reference to P, not a
  copy of P's text

### Requirement: Preferences and durable data are stored separately

The system SHALL store the settings surface as three kinds of document, in two
directories, distinguished by what the values are rather than by which screen
edits them:

- **Preferences** - the scalar values: everything the user tunes about how the
  app looks and behaves, plus the per-agent-type command and options maps, the
  font settings, the sidebar width, the source base folder and its
  first-launch detection flag, and the document version marker. These SHALL be
  stored as a single preferences document in the platform's user-preferences
  directory (on macOS, `~/Library/Preferences` under the application's
  directory).
- **Durable data** - the collections of objects the user created or that
  Knot recorded on their behalf: saved agents, saved workspaces, personas,
  bench templates, library prompts, recent repositories and recorded pull
  requests. These SHALL be stored in the platform's application-data
  directory (on macOS, `~/Library/Application Support` under the
  application's directory), as one document per collection: saved agents,
  workspaces, personas, bench templates, library prompts, recent repositories
  and recorded pull requests each in their own document.

  Recorded pull requests are durable data rather than a preference even though
  the user did not type them: they are a collection of objects with identity
  that grows without bound, which is what separates the two kinds here.
- **UI state** - what the application itself recorded about how its windows
  were last arranged, which the user never entered and would not miss if it
  were discarded. This SHALL be stored in the application-data directory, in
  its own document, separate from every durable-data collection.

A saved workspace SHALL hold only what the user configured about it: its
identity, its name, its color, and which agents belong to it. How that
workspace's window was last arranged is UI state and SHALL NOT be part of the
saved workspace record.

The two directories SHALL be derived from the same organization and
application identity the store already uses, so the preferences document and
the data documents name the same application.

On a platform where the two directories are the same, the split into separate
documents SHALL still hold; only their location coincides.

#### Scenario: Preferences live beside the platform's other preferences

- **WHEN** a scalar is written and the store's locations are inspected
- **THEN** the preferences document is under the platform's user-preferences
  directory, not the application-data directory

#### Scenario: Each collection is its own document

- **WHEN** a store holding agents, workspaces, personas, bench templates,
  library prompts, recent repositories and recorded pull requests is
  persisted
- **THEN** the application-data directory holds one document per collection,
  seven in all

#### Scenario: A fresh install writes only what it has

- **WHEN** a store with no persisted documents has a single scalar set
- **THEN** the preferences document is written and no collection document is
  required to exist for the store to load again

#### Scenario: A store with no recorded pull requests

- **WHEN** a store that predates the recorded-pull-request document is loaded
- **THEN** it loads successfully with no recorded pull requests, and no
  document for them is written until one is recorded

#### Scenario: A store with no prompt library

- **WHEN** a store that predates the library-prompt document is loaded
- **THEN** it loads successfully with an empty library, and no document for
  it is written until a prompt is added

#### Scenario: A saved workspace holds no UI state

- **WHEN** the workspaces document is written for a workspace whose window has
  been moved, resized, split and detached
- **THEN** the workspace's record holds its identity, name, color and agent
  membership, and none of that window arrangement
