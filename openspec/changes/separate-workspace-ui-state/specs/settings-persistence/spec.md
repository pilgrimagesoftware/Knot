# Spec Delta

## MODIFIED Requirements

### Requirement: Preferences and durable data are stored separately

The system SHALL store the settings surface as three kinds of document,
distinguished by what the values are rather than by which screen edits them:

- **Preferences** - the scalar values: everything the user tunes about how the
  app looks and behaves, plus the per-agent-type command and options maps, the
  font settings, the sidebar width, the source base folder and its
  first-launch detection flag, and the document version marker. These SHALL be
  stored as a single preferences document in the platform's user-preferences
  directory (on macOS, `~/Library/Preferences` under the application's
  directory).
- **Durable data** - the collections of objects the user created: saved
  agents, saved workspaces, personas, bench templates and recent
  repositories. These SHALL be stored in the platform's application-data
  directory (on macOS, `~/Library/Application Support` under the
  application's directory), as one document per collection: saved agents,
  workspaces, personas, bench templates and recent repositories each in their
  own document.
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

- **WHEN** a store holding agents, workspaces, personas, bench templates and
  recent repositories is persisted
- **THEN** the application-data directory holds one document per collection

#### Scenario: A fresh install writes only what it has

- **WHEN** a store with no persisted documents has a single scalar set
- **THEN** the preferences document is written and no collection document is
  required to exist for the store to load again

#### Scenario: A saved workspace holds no UI state

- **WHEN** the workspaces document is written for a workspace whose window has
  been moved, resized, split and detached
- **THEN** the workspace's record holds its identity, name, color and agent
  membership, and none of that window arrangement

## ADDED Requirements

### Requirement: Per-workspace UI state is its own document

The system SHALL store, per workspace and keyed by the workspace's identity,
the UI state the application tracks on the user's behalf: the window's saved
bounds, the layout mode, which agents are active in that layout, the focused
pane, the layout's split ratios, whether the dashboard is showing, and whether
the workspace is detached.

Writing any of that SHALL persist the UI-state document and SHALL NOT rewrite
the saved-workspaces document, the saved-agents document, or any other. Where a
window sits is the most frequently written value in the store and the least
valuable; it SHALL NOT be a reason to rewrite what the user configured.

Reading the settings surface SHALL NOT require knowing which document a value
came from, as it does not for any other value.

An entry SHALL be decode-tolerant field by field: a document written before a
field existed SHALL load with that field defaulted rather than failing, and an
entry that cannot be decoded at all SHALL cost only that workspace's
arrangement.

A missing or undecodable UI-state document SHALL leave every workspace loadable
with default arrangement. Losing this document SHALL never cost a workspace, an
agent, or a workspace's agent membership.

An entry keyed to a workspace that no longer exists SHALL be discarded rather
than retained indefinitely, so deleting a workspace does not leave its window
arrangement behind forever.

#### Scenario: Moving a window leaves user data alone

- **WHEN** the user drags a workspace's window to a new position
- **THEN** the UI-state document is written, and the saved-workspaces and
  saved-agents documents are byte-for-byte unchanged

#### Scenario: Arrangement survives a restart

- **WHEN** a workspace's window is moved, resized and split, and the app is
  restarted
- **THEN** the window reopens with the same bounds, layout mode and split
  ratios it had

#### Scenario: A missing UI-state document costs only arrangement

- **WHEN** the UI-state document is deleted and the app is started
- **THEN** every workspace loads with its name, color and agents intact, and
  its window opens with default arrangement

#### Scenario: An undecodable entry is contained

- **WHEN** the UI-state document holds one entry that cannot be decoded and
  others that can
- **THEN** the workspace with the bad entry opens with default arrangement and
  the others open with theirs

#### Scenario: A deleted workspace's arrangement is not kept

- **WHEN** a workspace is deleted and the store is reloaded
- **THEN** the UI-state document holds no entry for it

### Requirement: A combined workspace document is split once

An installation whose workspaces document holds UI state inside its workspace
records SHALL be migrated on load: each record's user-configured fields SHALL
become the saved workspace, its UI-state fields SHALL become that workspace's
UI-state entry, and both documents SHALL be written.

The migration SHALL preserve every value. A workspace's window bounds, layout
mode, active agents, focused pane, split ratios, dashboard visibility and
detached flag SHALL survive the move, so the first launch after upgrading opens
windows where the user left them.

The migration SHALL run only while a combined document is present, and SHALL
NOT run again once the documents are split. It SHALL compose with the migration
of the legacy single settings document: an installation still holding that
document SHALL arrive at both of these documents, not at a combined one.

A combined record missing some UI-state fields SHALL migrate with those fields
defaulted, on the same decode-tolerant terms as any other load.

#### Scenario: An upgraded installation keeps its windows

- **WHEN** an installation whose workspaces document carries window bounds and
  layout state is loaded by a version that stores them separately
- **THEN** the workspaces document holds only configured fields, the UI-state
  document holds the arrangement, and each workspace's window opens where it
  was

#### Scenario: The split does not run twice

- **WHEN** an already-split installation is loaded again
- **THEN** no migration is performed and neither document is rewritten by the
  load

#### Scenario: Migrating from the legacy single document

- **WHEN** an installation still holding the legacy single settings document is
  loaded
- **THEN** it arrives at separate preferences, collection and UI-state
  documents, with its workspaces' arrangement preserved
