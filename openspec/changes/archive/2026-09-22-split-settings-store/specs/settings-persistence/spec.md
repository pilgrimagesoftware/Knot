# Spec Delta

## MODIFIED Requirements

### Requirement: Single settings store

The system SHALL expose one settings surface holding scalar values (appearance
mode, restore-layout-on-launch, restore-conversation-on-launch,
keep-in-menu-bar, MCP enabled, MCP port default `8766`, source base folder,
notification toggle, markdown/mermaid view options, per-agent-type command and
options strings, terminal font name and size, autopilot-enabled, AI provider,
AI API key, autopilot action, autopilot custom prompt, voice-enabled, voice
engine, push-to-talk key code, voice auto-insert, and similar) and serialized
collections (saved agents, saved workspaces, personas, bench agents, recent
repos), including the compact tool-call display preference. Writing a value
SHALL persist it immediately.

One surface does not mean one document: the surface SHALL be backed by the
set of documents defined in "Preferences and durable data are stored
separately". A reader of the settings surface SHALL NOT have to know which
document a value came from.

`restore-conversation-on-launch` defaults to off (disabled) and is
decode-tolerant: a persisted settings blob written before this field existed
SHALL load with it defaulted to off, not an error.

`autopilot_enabled` defaults to `false`, `ai_provider` defaults to
`"openai"`, `ai_api_key` defaults to empty, `autopilot_action` defaults to
`"mark"`, and `autopilot_custom_prompt` defaults to empty. All five are
decode-tolerant: a persisted settings blob written before they existed SHALL
load with each defaulted, not an error.

`voice_enabled` defaults to `false`, `voice_engine` defaults to `"apple"`,
`voice_push_to_talk_key` defaults to `54` (Right Command, matching the
Swift reference's `ModifierKeyCode.rightCommand`), and `voice_auto_insert`
defaults to `true`. All four are decode-tolerant: a persisted settings blob
written before they existed SHALL load with each defaulted, not an error.

The compact tool-call display preference defaults to disabled, so an
existing install keeps the per-call rendering it already had. It is
decode-tolerant on the same terms: a persisted settings blob written before
it existed SHALL load with compact mode disabled, not an error.

#### Scenario: Compact mode persists

- **WHEN** the user enables compact tool-call mode and restarts the app
- **THEN** compact mode remains enabled

#### Scenario: Legacy settings default compact mode off

- **WHEN** a settings blob predates the compact tool-call preference
- **THEN** it loads successfully with compact mode disabled

#### Scenario: Scalar persists across restart

- **WHEN** the MCP port is set to `9000` and the app restarts
- **THEN** the MCP port reads back as `9000`

#### Scenario: Restore-conversation-on-launch defaults off for legacy settings

- **WHEN** a persisted settings blob written before `restore-conversation-on-launch`
  existed is decoded
- **THEN** it loads successfully with `restore-conversation-on-launch` set to
  off

#### Scenario: Autopilot scalars default for legacy settings

- **WHEN** a persisted settings blob written before the autopilot scalars
  existed is decoded
- **THEN** it loads successfully with `autopilot_enabled` false,
  `ai_provider` `"openai"`, `ai_api_key` empty, `autopilot_action`
  `"mark"`, and `autopilot_custom_prompt` empty

#### Scenario: Voice scalars default for legacy settings

- **WHEN** a persisted settings blob written before the voice scalars
  existed is decoded
- **THEN** it loads successfully with `voice_enabled` false, `voice_engine`
  `"apple"`, `voice_push_to_talk_key` `54`, and `voice_auto_insert` true

#### Scenario: One surface over several documents

- **WHEN** settings are loaded from a store whose preferences and collections
  are held in separate documents
- **THEN** every scalar and every collection reads back through the same
  settings surface, with no indication of which document supplied it

### Requirement: Decode-tolerant migration

Decoding a collection SHALL tolerate records that predate later fields:
`SavedAgent` and `BenchAgent` without created-by / is-companion / persona id
SHALL default those, and `SavedAgent` without session id SHALL default it to
`None`; a `Persona` without type / state SHALL default to user / enabled. A
collection document that fails to decode SHALL yield an empty collection
rather than an error, and SHALL NOT affect any other document (see
"A failed document is contained").

#### Scenario: Legacy persona record

- **WHEN** a stored persona has no `type` or `state`
- **THEN** it loads as a user persona in the enabled state

#### Scenario: Legacy SavedAgent record without session id

- **WHEN** a persisted `SavedAgent` record predates the session id field
- **THEN** it loads with session id defaulted to `None`

#### Scenario: Corrupt blob

- **WHEN** the saved-agents document cannot be decoded
- **THEN** the loaded agent list is empty and the app still starts

## ADDED Requirements

### Requirement: Preferences and durable data are stored separately

The system SHALL store the settings surface as two kinds of document, in two
directories, distinguished by what the values are rather than by which screen
edits them:

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

### Requirement: A write touches only the document it belongs to

Writing a value SHALL persist the document that value belongs to, and SHALL
NOT rewrite the documents it does not belong to. Setting a scalar SHALL write
the preferences document only; saving, editing or deleting a persona SHALL
write the personas document only; and likewise for agents, workspaces, bench
templates and recent repositories.

Persisting the whole surface at once SHALL remain available and SHALL write
every document.

#### Scenario: A scalar leaves the collections alone

- **WHEN** the MCP port is set on a store whose collection documents already
  exist on disk
- **THEN** the preferences document is rewritten and the collection documents
  are byte-for-byte unchanged

#### Scenario: Saving a persona leaves the agents alone

- **WHEN** a persona is saved
- **THEN** the personas document is rewritten and the agents, workspaces,
  bench and recent-repositories documents are byte-for-byte unchanged

#### Scenario: A whole-surface persist writes everything

- **WHEN** the whole settings surface is persisted
- **THEN** the preferences document and every collection document are written

### Requirement: A failed document is contained

Each document SHALL load independently. A document that is missing,
unreadable, not valid JSON, or not the expected shape SHALL yield that
document's defaults - empty for a collection, the documented defaults for
preferences - and SHALL NOT prevent any other document from loading, nor
prevent the app from starting.

#### Scenario: One corrupt collection

- **WHEN** the personas document holds unparseable content and the agents,
  workspaces and preferences documents are intact
- **THEN** personas load empty while agents, workspaces and every preference
  load exactly as written

#### Scenario: Corrupt preferences

- **WHEN** the preferences document holds unparseable content and the
  collection documents are intact
- **THEN** every scalar loads at its default and every collection loads
  exactly as written

### Requirement: Each document is written atomically

Writing any document SHALL write to a temporary file beside it and rename that
file into place, so a reader observes either the previous document or the new
one and never a partial write. A failed rename SHALL remove the temporary file
rather than leave it to shadow the next write.

#### Scenario: An interrupted write leaves the previous document

- **WHEN** a write to a collection document does not complete
- **THEN** the document on disk is the one written before it, intact, and no
  temporary file remains to be read in its place

### Requirement: The legacy single document is migrated once

The system SHALL migrate an installation that holds the legacy single settings
document (`settings.json` in the application-data directory). On load, when
that document is present, the system SHALL read it, distribute its values into
the preferences document and the collection documents, write them, and then
rename the legacy document to `settings.json.migrated` so it is never read
again.

Migration SHALL run only when the legacy document is present. Once renamed,
subsequent loads SHALL read the new documents alone.

A value already present in a new document SHALL win over the legacy
document's value: the new documents are the store, and the legacy document is
a one-time source for an installation that has none.

A legacy document that cannot be read or decoded SHALL be treated as absent -
the store loads at its defaults and the unreadable file SHALL be left in place
rather than renamed, so it can be recovered by hand.

The font-role migration and its `settingsVersion` marker SHALL survive this
move unchanged: the marker is a preference, it is carried into the preferences
document, and a legacy document written under the old font-role arrangement
SHALL have that migration applied exactly as before.

#### Scenario: An upgraded installation

- **WHEN** a store whose application-data directory holds only the legacy
  `settings.json`, carrying scalars and every collection, is loaded
- **THEN** every scalar and every collection reads back as it was written,
  the new documents exist on disk, and the legacy document has been renamed
  `settings.json.migrated`

#### Scenario: Migration does not run twice

- **WHEN** a store is loaded, migrated, and then loaded again
- **THEN** the second load reads the new documents, no migration runs, and no
  file named `settings.json` is recreated

#### Scenario: A legacy document alongside existing new documents

- **WHEN** a legacy `settings.json` and an already-written personas document
  are both present, and they disagree about the personas
- **THEN** the loaded personas are the ones from the personas document

#### Scenario: An unreadable legacy document

- **WHEN** the legacy `settings.json` holds unparseable content
- **THEN** the store loads at its defaults, the app starts, and
  `settings.json` is still on disk under that name

#### Scenario: Font roles are migrated out of a legacy document

- **WHEN** a legacy `settings.json` carrying no `settingsVersion` marker holds
  `uiFontName` `"Helvetica Neue"` and `titleFontName` `"Palatino"` is migrated
- **THEN** the written preferences document holds the UI font `"Palatino"`,
  the title font `"Helvetica Neue"`, and the current `settingsVersion` marker
