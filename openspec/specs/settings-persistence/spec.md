# settings-persistence Specification

## Purpose
Defines Knot's configuration and durable-object store: the scalar settings,
the serialized collections (saved agents, saved workspaces, personas, bench
templates, recent repos), the durable agent field set, first-launch
source-folder detection, the recent-repos MRU, and decode-tolerant migration.
One settings surface is backed by several documents - preferences apart from
durable data, one document per collection - and migrating an installation off
the single document it used to be is part of the contract. The Rust port MAY
use any backing store; the persisted shapes and behaviors are the contract.

## Requirements

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

### Requirement: The two proportional font settings name what they draw

The settings surface SHALL hold two proportional font settings, each named for
the text it governs:

- The UI font (`ui_font_name` / `ui_font_size`) is the application's default
  family and text size - the face the interface and all body text is drawn in.
  It defaults to `"Adamina"` at `16.0`.
- The title font (`title_font_name` / `title_font_size`) is the family and text
  size for titles and headers. It defaults to `"Manrope"` at `14.0`.

Both SHALL be decode-tolerant: a persisted document written before these
settings existed SHALL load with each defaulted, not an error.

Neither setting SHALL be the terminal font, which stays separate
(`terminal_font_name` / `terminal_font_size`).

#### Scenario: A fresh store carries the two roles at their defaults

- **WHEN** settings are loaded with no persisted document
- **THEN** the UI font is `"Adamina"` at `16.0` and the title font is
  `"Manrope"` at `14.0`

#### Scenario: Writing one font leaves the other alone

- **WHEN** the UI font family is set to `"Iowan Old Style"`
- **THEN** that value is persisted immediately and the title font's family and
  size are unchanged

### Requirement: A pre-migration document's font roles are swapped once

Before this change the two settings held the opposite roles: the value under
`uiFontName` / `uiFontSize` was applied to titles and secondary text, and the
value under `titleFontName` / `titleFontSize` was the application-wide default.

The system SHALL record which font-role arrangement a persisted document was
written under, and SHALL migrate a document written under the old arrangement
exactly once on load by exchanging the two families' values and the two sizes'
values. A document carrying no such marker SHALL be treated as written under
the old arrangement.

The migration SHALL exchange only values the document actually carries: a
persisted font setting that is absent SHALL stay absent and take the new
default, rather than receiving the other setting's value. A document that
customized neither font therefore loads at the new defaults, unchanged in
effect.

Once migrated, the loaded settings SHALL present the new arrangement, and the
next persist SHALL record that the migration has run so it cannot run twice.

The migration SHALL NOT apply to the terminal font.

#### Scenario: Both fonts were customized

- **WHEN** a document with no migration marker holds `uiFontName` `"Helvetica
  Neue"` at size `13` and `titleFontName` `"Palatino"` at size `18` is loaded
- **THEN** the loaded UI font is `"Palatino"` at `18.0` and the loaded title
  font is `"Helvetica Neue"` at `13.0`

#### Scenario: Only one font was customized

- **WHEN** a document with no migration marker holds `titleFontName`
  `"Palatino"` and no `uiFontName`
- **THEN** the loaded UI font is `"Palatino"` and the loaded title font is the
  new default `"Manrope"`

#### Scenario: Neither font was customized

- **WHEN** a document with no migration marker holds neither font key
- **THEN** the loaded UI font is `"Adamina"` at `16.0` and the loaded title
  font is `"Manrope"` at `14.0`

#### Scenario: The migration does not run twice

- **WHEN** a document that already carries the migration marker holds
  `uiFontName` `"Adamina"` and `titleFontName` `"Manrope"`
- **THEN** the loaded values are exactly those, with no exchange

#### Scenario: A migrated document is recorded as migrated

- **WHEN** a pre-migration document is loaded and then persisted
- **THEN** the written document carries the migration marker, and loading it
  again leaves the font values as they were written

### Requirement: Durable agent field set

A persisted agent (`SavedAgent`) SHALL store exactly: id, name, avatar (a
non-empty string, default a robot emoji), folder, agent type (default
`claude`), created-by, is-companion, shell command, persona id, and an
optional session id. Loading SHALL reconstruct agents with all runtime fields
at defaults except session id, which SHALL be reconstructed from the
persisted value when present (see `agent-lifecycle` - Durable versus runtime
fields for how it is then used to resolve resume-session id).

Session id SHALL only ever be written to a non-`None` value when
`restore-conversation-on-launch` is enabled at persist time; whenever it is
disabled, saving an agent SHALL persist session id as `None` regardless of
the agent's runtime session id.

#### Scenario: Avatar default on save

- **WHEN** an agent with no avatar is saved
- **THEN** its persisted avatar is the default robot emoji

#### Scenario: Session id omitted by default

- **WHEN** `restore-conversation-on-launch` is disabled (the default) and an
  agent with an active runtime session id is saved
- **THEN** the persisted agent's session id is `None`

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

### Requirement: First-launch source-folder detection

On first launch with no source base folder set, the system SHALL pick the
first existing directory among the common source locations (`~/src`,
`~/source`, `~/sources`) and record it, marking detection done so it does not
run again.

#### Scenario: Picks the first that exists

- **WHEN** `~/src` is absent but `~/source` exists on first launch
- **THEN** the source base folder becomes `~/source`

### Requirement: Recent repositories MRU

The system SHALL maintain a recent-repositories list as a bounded
most-recently-used sequence: adding an entry moves it to the front and
de-duplicates.

#### Scenario: Re-adding moves to front

- **WHEN** repo `B` is added while the list is `[A, B, C]`
- **THEN** the list becomes `[B, A, C]`

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

### Requirement: The workspace sidebar's width is a stored scalar

The settings surface SHALL hold the workspace sidebar's width as a scalar,
`sidebar_width`, defaulting to `250.0`. Writing it SHALL persist it immediately,
as with every other scalar.

It SHALL be decode-tolerant: a persisted document written before the field
existed SHALL load with it defaulted, not as an error.

A persisted value outside the sidebar's permitted range SHALL be brought into
range on load - clamped to the nearest bound - rather than rejected, loaded as
is, or reset to the default. A document hand-edited to `40` describes a
narrower sidebar than the window allows, and the nearest legal width is the
closer answer to that intent than 250 is.

A persisted value that is not a number SHALL leave the setting at its default,
as any other undecodable scalar does.

One width SHALL apply to every workspace, unlike a workspace's window bounds,
which are stored per workspace.

#### Scenario: A fresh store

- **WHEN** settings are loaded with no persisted document
- **THEN** `sidebar_width` is `250.0`

#### Scenario: A document from before the setting existed

- **WHEN** a document holding other scalars but no `sidebarWidth` is loaded
- **THEN** it loads without error and `sidebar_width` is `250.0`

#### Scenario: A persisted width is honored

- **WHEN** a document holding `sidebarWidth` `320` is loaded
- **THEN** `sidebar_width` is `320.0`

#### Scenario: A width below the minimum

- **WHEN** a document holding `sidebarWidth` `40` is loaded
- **THEN** `sidebar_width` is the minimum permitted width

#### Scenario: A width above the maximum

- **WHEN** a document holding `sidebarWidth` `5000` is loaded
- **THEN** `sidebar_width` is the maximum permitted width

#### Scenario: Writing the width persists it

- **WHEN** `sidebar_width` is set to `180.0`
- **THEN** the value is written immediately and a subsequent load reports
  `180.0`

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
