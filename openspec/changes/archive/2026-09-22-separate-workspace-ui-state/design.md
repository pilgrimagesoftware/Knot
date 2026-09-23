# Design

## Context

See proposal.md — Why. What the store already does, and what it does not:

- `StorePaths` holds two directories and derives every document path from them,
  so nothing outside it knows a filename. Adding a document is adding a method
  there plus a constant.
- `documents.rs` writes every document atomically (temp file beside the target,
  then rename) and reads tolerantly, yielding defaults for anything it cannot
  make sense of. A new document inherits both for free.
- `legacy.rs` migrates the single `settings.json` on load, and is idempotent
  **without a marker** by the rule "the new document wins": if the process dies
  part-way, the next launch takes what was already written and finishes. Its
  other decision matters here too — it reads the legacy document as a `Value`
  rather than through a struct, because a struct would be a second copy of the
  field list and a field in one and not the other would silently drop a setting.
- `AgentStore` holds `Vec<knot_core::Workspace>` directly and hands it out as
  `&[Workspace]`, so the persisted record is also the runtime record. Splitting
  one splits the other.
- The eight fields have roughly 100 references across the workspace, the largest
  being `active_agent_ids` (20) and `window_bounds` (19).

## Goals / Non-Goals

**Goals:**

- No value lost in the move, on any upgrade path, including an installation
  still on the legacy single document.
- A UI-state write that costs one small document and nothing else.

**Non-Goals:**

- A general "state vs configuration" framework. Two record kinds, one new
  document.
- Fixing the window's stale `Settings` snapshot (issue #238).

## Decisions

### Detect a combined document by inspecting its keys, not by decoding it

This is the decision the change turns on. Serde ignores unknown fields by
default, so a combined `workspaces.json` will decode cleanly into the narrowed
`Workspace` — dropping every arrangement the user had, with no error, no
warning, and nothing to notice until windows open centered. A migration that
runs after decoding cannot see what it already lost.

So the workspaces document is read as a `Value` first and examined for the UI
keys, exactly as `legacy.rs` reads the legacy document as a `Value` and for a
closely related reason. Only then is it decoded into records.

Idempotence follows `legacy.rs`'s rule rather than inventing a marker: an entry
already present in the UI-state document wins over the same workspace's combined
fields. Write the UI-state document first, then rewrite the workspaces document
without the UI keys — which is what clears the "combined" condition. A crash
between the two leaves a document that still looks combined, and the next launch
finds the UI entries already written and finishes without overwriting them.

### One document holding a map keyed by workspace id

`workspace-ui-state.json` in the application-data directory, holding
`BTreeMap<Uuid, WorkspaceUiState>`.

The alternative — a list of records each carrying a `workspace_id`, matching
every other collection document and reusing `write_collection` — was rejected on
what the document is *for*. Every read is "the state for this workspace", every
write is "this workspace's state", and pruning orphans is a key operation. A map
does all three directly; a list does each of them by scan and admits duplicate
ids the type then has to rule out. The other documents are lists because they
are collections the user browses; this one is a lookup table.

Orphans are pruned on load, against the workspaces actually present, rather than
on workspace deletion. Deletion already has several paths and a missed one would
leak entries silently, where load is a single funnel every path goes through.

### `AgentStore` gains a parallel map, and the setters move to it

`set_workspace_window_bounds` and its siblings operate on the UI map rather than
on `workspace_mut`. Callers reading `workspace.window_bounds` and the other seven
fields stop compiling, which is the point: roughly 100 references is too many to
audit by eye, and the compiler is exhaustive where a grep is not.

This does add a second per-key map to a struct that already has too many, which
the parallel-maps rule warns about. It is accepted here because the two have
genuinely different lifetimes — a workspace's configuration outlives any window,
and its arrangement is meaningless without one — and because the pruning is on
load rather than spread across teardown paths. Prune in one place and say so.

### The bounds observer writes the UI document alone

`workspace_window/open.rs` currently answers a bounds change with
`persist_agents`, which copies the store into the window's `Settings` snapshot
and writes both `agents.json` and `workspaces.json`. It should write the
UI-state document only, and should not route through the snapshot to do it.

This is where the change pays for itself, and it is the one call site worth
changing carefully rather than mechanically.

## Risks / Trade-offs

- **Silent loss of every user's window arrangement if the detection is wrong.**
  The failure mode is invisible: no error, windows simply open centered. The
  test that matters is a real combined `workspaces.json` fixture — one captured
  from an actual installation, not hand-written to match the code — loaded and
  checked field by field. Write that test before the migration, not after.
- **Two migrations that must compose.** An installation on the legacy
  `settings.json` has to end at both new documents. The legacy migration lifts
  `savedWorkspaces` out of the legacy document and writes `workspaces.json`;
  that output is itself combined, so the split must run after it and on its
  result, not only on a pre-existing `workspaces.json`. Test the two-step path
  explicitly; it is the one no one will try by hand.
- **Issue #238 survives.** The window still holds a stale `Settings` snapshot,
  and a roster write still reverts settings-window edits. This removes the
  most frequent trigger, which will make the bug rarer and therefore harder to
  reproduce. Say so on the issue rather than letting it look fixed.
- **A second per-key map on `AgentStore`.** See the decision above. Keep pruning
  in the one place, and add the map to any teardown that already prunes per-key
  state so the two rules do not disagree.
- **Around 100 call sites move.** All compiler-found, most mechanical. The risk
  is not missing one, it is an accidental behavior change while touching that
  many lines. Keep the field semantics identical; anything that looks worth
  improving on the way past belongs in a different change.
