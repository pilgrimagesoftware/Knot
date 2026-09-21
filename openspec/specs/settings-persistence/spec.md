# settings-persistence Specification

## Purpose
Defines Knot's single configuration and durable-object store: the scalar
settings, the serialized collections (saved agents, saved workspaces, personas,
bench templates, recent repos), the durable agent field set, first-launch
source-folder detection, the recent-repos MRU, and decode-tolerant migration.
The Rust port MAY use any backing store; the persisted shapes and behaviors are
the contract.

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
collection blob that fails to decode SHALL yield an empty collection rather
than an error.

#### Scenario: Legacy persona record

- **WHEN** a stored persona has no `type` or `state`
- **THEN** it loads as a user persona in the enabled state

#### Scenario: Legacy SavedAgent record without session id

- **WHEN** a persisted `SavedAgent` record predates the session id field
- **THEN** it loads with session id defaulted to `None`

#### Scenario: Corrupt blob

- **WHEN** the saved-agents blob cannot be decoded
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
folder, agent type, optional shell command, optional persona id). Adding a
bench entry SHALL replace any existing entry with the same folder.

#### Scenario: Same-folder bench entry replaced

- **WHEN** a bench entry is added for a folder that already has one
- **THEN** the old entry is removed and only the new one remains
