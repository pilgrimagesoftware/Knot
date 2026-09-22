# Design

## Context

See `proposal.md` - Why. The constraints that shape the approach:

- `Settings` is one flat `#[serde(default, rename_all = "camelCase")]` struct
  in `crates/knot-core/src/settings/store.rs` (531 lines), read and mutated
  from roughly 30 call sites across `knot`, `knot-terminal` and
  `knot-mcp-tools`. Its field access is ubiquitous (`settings.mcp_server_port`,
  `settings.personas`), so any change to that shape is a change to all of them.
- The store path comes from `ProjectDirs::from(ORG_QUALIFIER, ORG_NAME,
  APP_NAME).config_dir().join(SETTINGS_FILE)`, held as
  `store_path: Option<PathBuf>` on the struct (`#[serde(skip)]`), with a
  `with_store_path` constructor tests use.
- `persist` already writes to a sibling temp file and renames it into place.
- Two load-time upgrades run today: the `"SF Mono"` terminal font replacement
  and the `settingsVersion`-gated font-role swap.
- `make size-check` fails any `.rs` file over 700 lines, so `store.rs` cannot
  absorb this.

## Goals / Non-Goals

**Goals:**

- Leave every call site untouched. The field set on `Settings` and the way it
  is read stay exactly as they are.
- Name each document's fields in exactly one place, so adding a setting or a
  collection later cannot desynchronise a mapping.
- Make the migration re-runnable: an interrupted migration finishes correctly
  on the next launch.

**Non-Goals:**

- Splitting `Settings` into separate in-memory types (`Preferences`,
  `AgentStore`, ...). That is a larger refactor with its own case; this change
  is about where bytes land.
- Dirty-tracking or batching writes. Each mutating helper keeps writing
  immediately; it just writes less.

## Decisions

### Serialize `Settings` itself as the preferences document

Mark the five collection fields `#[serde(skip)]` and serialize `Settings` as
the preferences document. The preferences document is then literally today's
document minus the collection keys, and each collection is serialized on its
own as a bare JSON array (`[ {...}, {...} ]`).

*Why over the alternatives:*

- A separate `PreferencesDocument` struct with `From<&Settings>` conversions
  would restate ~35 field names and their serde attributes in a second place.
  Adding a setting and updating only one side is a silent data-loss bug, and
  nothing in the type system catches it.
- Nesting the scalars behind `settings.prefs.*` would touch every call site
  for no behavioral gain.

`#[serde(skip)]` means a stray `savedAgents` key in a preferences document is
ignored on read and never written - which is the behavior wanted, since the
collections have moved.

A bare array per collection, rather than an object wrapper like
`{"personas": [...]}`, because the document's name already says what it holds;
a wrapper adds a key to get wrong and a nesting level to hand-edit past.

### `StorePaths` replaces `store_path`

`store_path: Option<PathBuf>` becomes `paths: Option<StorePaths>`
(`#[serde(skip)]`), where `StorePaths` holds the preferences file path and the
data directory, and derives each document's path from the data directory. It
carries the platform derivation:

- preferences: `ProjectDirs::preference_dir()` (macOS
  `~/Library/Preferences/com.Pilgrimage-Software.Knot`)
- data: `ProjectDirs::config_dir()` (macOS `~/Library/Application
  Support/com.Pilgrimage-Software.Knot`) - unchanged from today, so no data
  document moves.

`Settings::with_store_path(path_to_settings_json)` is replaced by
`Settings::with_store_root(dir)`, which puts both the preferences document and
the collection documents under one directory. Tests get a one-line constructor
and a single `TempDir` to assert against; the five call sites that build a
store from a `settings.json` path are updated.

On Linux and Windows `preference_dir()` and `config_dir()` are the same
directory. The documents still have distinct names, so they coexist; only
their location coincides, which the spec allows.

### Per-collection write helpers, `persist` fans out

Each collection gets a private writer (`persist_personas`, `persist_agents`,
...) sharing one atomic write-and-rename helper. The mutating helpers call the
one that matches what they changed; `persist()` calls all six. Scalar setters
call `persist_preferences`.

The atomic write helper is the existing `persist` body, parameterized by path
and bytes - the temp-file-beside-the-target and remove-on-failed-rename
behavior is unchanged, just applied six times instead of once.

### Migration reads the legacy document as a `Value`, not a struct

On load, if `<data_dir>/settings.json` exists:

1. Parse it to `serde_json::Value`.
2. Take the five collection keys (`savedAgents`, `savedWorkspaces`,
   `personas`, `benchAgents`, `recentRepos`) out of the object and decode each
   with the existing tolerant per-record decoder.
3. Decode the remaining object into `Settings` through the same path a
   preferences document takes - font-role migration, `"SF Mono"` replacement
   and sidebar clamp included. Unknown keys are ignored by serde, so the
   collection keys left behind cost nothing.
4. For each collection, keep what the new document already holds if that
   document exists; otherwise take the legacy value.
5. Write all six documents, then rename `settings.json` to
   `settings.json.migrated`.

*Why a `Value` rather than a legacy struct:* the legacy shape is the current
`Settings` shape. A `LegacyDocument` struct would be a second copy of the same
field list, with the same desynchronisation hazard the preferences decision
rejects. Lifting five known keys out of a `Value` needs no such copy.

*Why step 4 (new documents win):* it makes the migration idempotent and
crash-safe without a marker. If the process dies after writing `personas.json`
but before the rename, the next launch finds the legacy document still
present, takes personas from the already-written document and everything else
from the legacy document, and finishes. A "legacy wins" rule would instead
overwrite whatever the user changed between the crash and the relaunch.

An unreadable legacy document is left under its own name rather than renamed,
so a user can inspect it. Treating it as absent (the store loads at defaults)
matches what the current code already does with an undecodable document.

### Module split

`store.rs` is already 531 lines and this adds path derivation, six writers and
the migration. Split under the existing `settings/store/` directory:

| Module | Holds |
|---|---|
| `store.rs` | the `Settings` type, defaults, accessors and mutating helpers |
| `store/paths.rs` | `StorePaths`, platform derivation, per-document filenames |
| `store/documents.rs` | read/write of one document, atomic rename, tolerant decode |
| `store/legacy.rs` | the one-time migration from `settings.json` |
| `store/tests.rs` | existing tests, updated for `with_store_root` |
| `store/legacy/tests.rs` | migration tests |

Filenames and the migrated suffix go in `knot-core/src/consts.rs` with the
rest, replacing `SETTINGS_FILE`.

## Risks / Trade-offs

- **A JSON file in `~/Library/Preferences` is not the macOS convention** -
  that directory holds plists managed by `cfprefsd`, and a foreign file there
  is invisible to `defaults`. → Knot has never used `CFPreferences`, so
  nothing regresses; the file is named `preferences.json` so it is obviously
  not a plist, and `cfprefsd` ignores files it did not write. Adopting
  `CFPreferences` properly is a separate change (proposal - Non-Goals).

- **Migration runs on a document holding everything the user owns** - a bug
  that drops a collection loses agents, workspaces and personas at once. → The
  legacy document is renamed, not deleted, so the data is recoverable by hand;
  migration is covered by tests over a document carrying every collection and
  every scalar; and the round-trip (migrate, then load) is asserted
  field-by-field rather than by sampling.

- **A downgrade after migration silently starts at defaults** - an older build
  looks for `settings.json` and finds `settings.json.migrated`. → Accepted and
  called out as breaking in the proposal. Rollback is a rename back.

- **Six writes where there was one** - a whole-surface `persist()` now does six
  create/write/rename cycles. → `persist()` is rare; the common path is a
  single mutating helper, which now writes one small document instead of the
  whole store, so the ordinary case gets cheaper, not more expensive.

- **Partially-written store on first run of the new build** - if the process
  dies mid-migration the data directory holds a mix of new documents and the
  legacy one. → That is the state step 4 is designed for; it is a tested
  scenario, not an edge case left to chance.

## Migration Plan

No deployment steps - the migration is in-process and runs on first load after
upgrade. Rollback for a user who needs the previous build: quit Knot, rename
`~/Library/Application Support/com.Pilgrimage-Software.Knot/settings.json.migrated`
back to `settings.json`, and delete the new documents alongside it and the
preferences document under `~/Library/Preferences/`. Changes made in the new
build after migration are lost by that rollback, which is why the renamed file
is kept rather than deleted.
