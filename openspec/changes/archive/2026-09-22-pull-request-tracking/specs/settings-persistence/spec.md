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
repos, recorded pull requests), including the compact tool-call display
preference. Writing a value
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
- **Durable data** - the collections of objects the user created or that
  Knot recorded on their behalf: saved agents, saved workspaces, personas,
  bench templates, recent repositories and recorded pull requests. These SHALL
  be stored in the platform's application-data directory (on macOS,
  `~/Library/Application Support` under the application's directory), as one
  document per collection: saved agents, workspaces, personas, bench templates,
  recent repositories and recorded pull requests each in their own document.

  Recorded pull requests are durable data rather than a preference even though
  the user did not type them: they are a collection of objects with identity
  that grows without bound, which is what separates the two kinds here.

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
  recent repositories and recorded pull requests is persisted
- **THEN** the application-data directory holds one document per collection,
  six in all

#### Scenario: A fresh install writes only what it has

- **WHEN** a store with no persisted documents has a single scalar set
- **THEN** the preferences document is written and no collection document is
  required to exist for the store to load again

#### Scenario: A store with no recorded pull requests

- **WHEN** a store that predates the recorded-pull-request document is loaded
- **THEN** it loads successfully with no recorded pull requests, and no
  document for them is written until one is recorded
