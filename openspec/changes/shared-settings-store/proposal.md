# Proposal

## Why

Every window holds its own `knot_core::Settings` by value. Seven structs do:
the settings window, workspace window, workspace manager, import window,
command centre, agent editor, and the agent-row menu targets. Each copy is
taken when its window opens and is a snapshot from that moment on.

Two things follow, and they are the two halves of issue #238.

`preferences-reach-open-windows` fixed the read half by broadcasting a refresh
after the settings window writes. That broadcast reaches workspace windows
only - `settings_broadcast::preferences_changed` iterates
`WindowRegistry::workspace_views`. The import window, workspace manager,
command centre and agent editor still draw values that stopped being true the
moment the user changed them.

The write half is unfixed. `knot-core`'s import paths take a caller's snapshot
and write the whole surface back over every document:
`crates/knot-core/src/import/personas.rs:50` and
`crates/knot-core/src/import/workspaces.rs:54` both call `Settings::persist()`.
The import window's snapshot is loaded when that window opens
(`crates/knot/src/import_window/window.rs:208`) and never refreshed, so a
preference changed in the settings window while the import window is open is
silently reverted by the next import.

The defect is narrower than issue #238 describes, because
`split-settings-store` already moved the roster writes to per-document
persists - a rename no longer reverts a setting. What remains is the snapshot
model itself, and it is guarded today by four hand-written reloads
(`workspace_window/sidebar_layout.rs:54`, `workspace_window/menus/agent_row.rs:112`
and `:352`, `agent_editor/window.rs:56`) that re-read from disk before writing.
Four call sites know about the hazard. The other holders do not, and nothing
makes a new window's author aware of it.

`Settings::reload_preferences`, the read half's refresh, carries its own
hazard: it decodes the preferences document and transplants the `#[serde(skip)]`
collections from the old value. A collection added to the struct and forgotten
in the transplant is silently cleared on the next preference change. A test
holds the two in step; nothing else does.

## What Changes

- The settings surface becomes one shared value rather than one copy per
  window. Every holder reads the same settings, so there is no snapshot to go
  stale and no write that can revert a value its holder never set.
- Sharing is copy-on-write, not a mutex. A reader takes a cheap handle to the
  current value; a writer builds a new value and swaps it in. The render path
  reads settings (`workspace_window/render/mod.rs:350`) and GPUI re-renders per
  keystroke, so a lock acquired per read is not acceptable there. Copy-on-write
  also keeps the settings surface out of any lock-ordering relationship with
  `AgentStore`'s mutex, which fifteen call sites already hold while reading
  settings.
- The refresh machinery is removed, not extended: `settings_broadcast`,
  `Settings::reload_preferences` and its `#[serde(skip)]`-transplant test, and
  the four hand-written reloads all become unnecessary and are deleted.
- **BREAKING** for callers inside the workspace: `Settings` is no longer held
  or passed by value. This is internal API only - nothing about the stored
  documents, their layout, or their contents changes.

## Capabilities

### New Capabilities

None. This changes how the existing surface is held, not what it holds.

### Modified Capabilities

- `settings-persistence`: the settings surface is shared rather than
  snapshotted per window, so a preference change applies everywhere without a
  delivery step, and a write from any holder cannot revert a value that holder
  did not set. The existing "A preference change applies to open windows"
  requirement stops describing a refresh and describes the absence of stale
  copies; a new requirement states that a write preserves values written
  elsewhere since the writer read them.

## Impact

- `knot-core`: `Settings` gains a shared handle type and a swap-on-write path.
  `reload_preferences` is removed. The import paths write the documents they
  touched rather than the whole surface.
- `knot`: seven structs stop owning a `Settings`; 106 read sites across 33
  files move to the shared handle. `settings_broadcast` is deleted.
  `app_bootstrap` threads the shared value instead of cloning snapshots.
- One new workspace dependency for the copy-on-write cell.
- No change to what is stored, to the document layout, or to migration.

## Non-goals

- Changing what is stored or how documents are laid out on disk. The document
  set, the migrations and the per-document write granularity from
  `split-settings-store` all stay exactly as they are.
- Making `Settings` mutable in place. Writers replace the value; they do not
  hand out mutable references to a shared one.
- Reworking which screen edits which value, or the settings UI.
- Live-reloading settings when the file changes underneath the app. The shared
  value is the source of truth for the running process; watching the file is a
  separate concern and not one this change needs.
