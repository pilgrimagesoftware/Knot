# Proposal

## Why

Every persisted value in Knot lives in one `settings.json` under
`~/Library/Application Support`: the user's preferences and the durable objects
they created (agents, workspaces, personas, bench templates, recent repos).
One document means a single corrupt or unreadable blob loses all of it at
once, every keystroke-level preference write rewrites the whole collection set,
and a user who wants to inspect, back up or hand-edit their personas has to
find them inside a document dominated by unrelated scalars.

The single document also puts preferences in the wrong place. macOS separates
user preferences (`~/Library/Preferences`) from application data
(`~/Library/Application Support`); Knot files both under the latter, so a
user clearing their preferences to recover from a bad font or window setting
would have to delete the file holding every agent and workspace they own.

## What Changes

- Split the one persisted document into a preferences document and one
  document per durable collection.
- Move the preferences document out of `~/Library/Application Support` to
  `~/Library/Preferences`, the platform's directory for user preferences.
  The durable collections stay in `~/Library/Application Support`.
- Data documents, one per collection: `agents.json` (saved agents),
  `workspaces.json`, `personas.json`, `bench.json` (bench templates) and
  `recent-repos.json`.
- Persist per document rather than per store: saving a persona writes
  `personas.json` only, and setting a scalar writes `preferences.json` only.
- Migrate an existing `settings.json` on first load after upgrade: split it
  into the new documents, then rename it `settings.json.migrated` so the
  split is recoverable by hand and never re-read.
- Contain the blast radius of a bad document: an unreadable `personas.json`
  yields empty personas and leaves agents, workspaces and preferences intact,
  where today one bad byte loses everything.
- **BREAKING** (on-disk only): `settings.json` is no longer the store. A
  downgrade to an older build after migration reads the renamed file as
  absent and starts at defaults. The in-process API (`Settings`) keeps its
  shape, so no caller changes.

## Capabilities

### New Capabilities

None. This redistributes an existing capability's storage rather than adding
behavior.

### Modified Capabilities

- `settings-persistence`: the "Single settings store" requirement becomes a
  set of documents with a defined split between preferences and durable
  collections, each in its platform-appropriate directory, each written and
  each decode-tolerant on its own. Adds a one-time migration requirement from
  the legacy single document.

## Impact

- `crates/knot-core/src/settings/store.rs` — load, persist and path
  derivation fan out across documents; `store.rs` is already 531 lines and
  will need splitting under the 700-line rule.
- `crates/knot-core/src/consts.rs` — `SETTINGS_FILE` replaced by one constant
  per document plus the legacy name and the migrated suffix.
- `directories` (already a dependency) — `ProjectDirs::preference_dir()`
  alongside `config_dir()`.
- `crates/knot-core/src/settings/store/tests.rs`,
  `crates/knot-core/tests/settings.rs`, `crates/knot/src/tests/`,
  `crates/knot-mcp-tools/src/lib.rs` — tests that build a store from a single
  `settings.json` path need a directory-rooted equivalent.
- ~30 call sites across `knot`, `knot-terminal` and `knot-mcp-tools` read and
  mutate `Settings`. The type's public surface is unchanged, so these are
  untouched apart from the test constructors above.

## Non-Goals

- No change to the shape of any persisted record. `SavedAgent`, `Workspace`,
  `Persona` and `BenchAgent` serialize exactly as they do today.
- No change to which values exist or to any default.
- No change to the font-role migration or the `settingsVersion` marker; it
  moves to the preferences document and keeps running on the same terms.
- No move to a platform preferences API (`CFPreferences`/`defaults`). The
  preferences document stays a JSON file Knot reads and writes itself; only
  its directory changes.
- No split of conversation history or session setup, which already persist
  outside this store.
