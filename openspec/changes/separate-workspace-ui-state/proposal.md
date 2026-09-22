# Proposal

## Why

`workspaces.json` holds two unrelated kinds of value in one record. Four fields
are the user's: the workspace's id, its name, its color, and which agents belong
to it. Eight are the app's own notes about how a window happened to be arranged
last time: `layout_mode`, `active_agent_ids`, `focused_pane_index`,
`split_ratio`, `split_ratio_secondary`, `show_dashboard`, `is_detached` and
`window_bounds`.

Mixing them is not only untidy, it is what puts user data in the path of a
pointer drag. Moving a workspace window fires the bounds observer, which calls
`persist_agents`, which rewrites **both** `agents.json` and `workspaces.json`
from the `Settings` snapshot the window is holding. So the highest-frequency,
lowest-value write in the app — where a window sits — is also a full rewrite of
the roster the user built. That coupling is the mechanism behind issue #238: a
settings-window edit is reverted by "the next roster write", and dragging a
window is a roster write.

The store already separates values by what they are rather than by which screen
edits them — preferences in one directory, the user's collections in another.
This applies the same distinction one level down, where it was missed.

## What Changes

- The saved workspace record keeps only what the user configured: id, name,
  color and agent membership.
- Per-workspace UI state moves to a document of its own, keyed by workspace id,
  holding the eight fields above.
- Writing UI state writes only the UI-state document. Where a window sits stops
  rewriting saved agents and saved workspaces.
- An existing `workspaces.json` carrying both is split once on load, and the
  result is written to both documents. Nothing the user configured and nothing
  the app remembered is lost in the move.
- UI state for a workspace that no longer exists is dropped rather than kept
  forever, and a missing or undecodable UI-state document costs only window
  arrangement — workspaces still load.

Non-goals:

- **Fixing issue #238.** The window's stale `Settings` snapshot is a separate
  defect, and it will still be a stale snapshot afterwards. This removes the
  most frequent thing that triggers it — a drag no longer writes the roster —
  and does not make the snapshot correct. Do not close #238 on the strength of
  this change.
- **The saved agent record, which mixes the same two kinds.** `SavedAgent`
  carries the user's configuration (name, avatar, folder, agent type, persona,
  activation mode, registry metadata) alongside the app's session bookkeeping
  (`session_id`, `acp_session_id`, `session_config`, and arguably `view_mode`).
  It is the same argument and deserves the same treatment, in its own change —
  the migration is the risky part and doing two records at once doubles it.
- The sidebar width, which is already a preference scalar in the preferences
  document, correctly separated and deliberately global rather than
  per-workspace.
- Any change to what the UI state *means* — layouts, detaching, dashboard
  visibility and bounds restoration all behave exactly as they do now.

## Capabilities

### Modified Capabilities

- `settings-persistence`: the requirement defining how documents are divided
  gains a third kind alongside preferences and durable data — the app's own UI
  state — and the workspace collection's record is narrowed to what the user
  configured. A new requirement covers the per-workspace UI-state document: what
  it holds, that writing it touches nothing else, how an existing combined
  document is migrated once, and what happens when it is missing or holds an
  entry for a workspace that is gone.

### New Capabilities

None. `settings-persistence` already owns how the store is divided.

## Impact

- `crates/knot-core/src/settings/records.rs` — `Workspace` loses eight fields;
  a new record holds them.
- `crates/knot-core/src/settings/store.rs` and `store/documents.rs`,
  `store/paths.rs` — a seventh document, its path, its loader and its writer.
  The module doc's document table is part of the change, not an afterthought.
- `crates/knot-core/src/settings/store/legacy.rs` — already migrates the single
  `settings.json`; this adds a second, independent one-way split. The two must
  compose: an installation still on `settings.json` has to arrive at both new
  documents.
- `crates/knot-agents/src/store/workspace.rs` — `set_workspace_window_bounds`
  and the other UI-state setters move off the `Workspace` record.
- `crates/knot/src/workspace_window/open.rs` — the bounds observer calls
  `persist_agents`; it should call a UI-state write instead. This is where the
  benefit actually lands.
- `crates/knot/src/workspace_manager/mod.rs`, `crates/knot/src/tests/mod.rs`,
  `crates/knot/src/tests/import_window.rs` — construct `Workspace` literals and
  will not compile until updated. The compiler finds these; do not hunt them by
  hand.
- `crates/knot-core/src/import/workspaces.rs` — importing a workspace from
  another tool creates configuration, not UI state. Check what it sets.
- No new dependency. No user-facing text. No change to either directory.
