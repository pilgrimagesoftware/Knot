# Design

## Context

See `proposal.md` - Why. The constraints that shape the approach:

- The render path reads settings. `workspace_window/render/mod.rs:350` reads
  two font values mid-render and GPUI re-renders per keystroke.
  `.claude/rules/rust-structure.md` bans I/O there, and a lock acquired per
  read is the same class of hazard for the same reason.
- Fifteen call sites hold `AgentStore`'s `parking_lot::Mutex` while reading
  settings - `workspace_manager/actions.rs:21-24` is the clearest. Any second
  lock on the settings surface enters a lock-ordering relationship with that
  one.
- `knot-core` cannot depend on `gpui`, and two of the offending writers live
  in `knot-core` (`import/personas.rs`, `import/workspaces.rs`). The shared
  handle therefore cannot be a GPUI global alone.
- `arc-swap` 1.9.2 is already in `Cargo.lock` as a transitive dependency, so
  promoting it to a direct workspace dependency adds no new supply-chain
  surface.
- GPUI globals are an established pattern in this crate: `QuitGuard`,
  `Theme` and `WindowRegistry` are all held that way.

## Goals / Non-Goals

**Goals:**

- One settings value for the running process, with no per-window copy that can
  diverge from it.
- Lock-free reads, safe to call from a render.
- A write that cannot revert a value the writer did not set.
- Net deletion of the machinery the snapshot model required.

**Non-Goals:**

- Mutable shared access. Writers replace the value; no caller gets a `&mut` to
  a shared `Settings`.
- Changing document layout, migrations, or the per-document write granularity
  from `split-settings-store`.
- Watching the settings files for external change.
- Cross-process coordination. Two Knot processes writing the same files is out
  of scope, as it is today.

## Decisions

### Copy-on-write over a mutex

`ArcSwap<Settings>` in `knot-core`. A reader calls `load_full()` and gets an
`Arc<Settings>` it can hold for as long as it likes; a writer builds the next
value and swaps it in.

Chosen over `Arc<Mutex<Settings>>` because of the render path. A mutex makes
every one of the 106 read sites a lock acquisition, and with reads nested
inside renders that call further code that also reads settings, a guard held
across a sub-call is a deadlock rather than a slowdown. Copy-on-write removes
the question: there is no guard to hold, so there is no ordering relationship
with `AgentStore`'s mutex and no new rule for a future author to learn.

Chosen over a GPUI global alone because `knot-core`'s import paths write
settings and cannot see `gpui`. The handle lives in `knot-core`; `knot`
registers it as a global so windows reach it the way they already reach
`WindowRegistry`.

The cost is that each write clones the whole `Settings`. That is a handful of
`Vec`s cloned on an explicit user action - saving a preference, importing,
editing the roster - and never on a frame.

### Reads take a snapshot handle, deliberately

`load_full()` returns an `Arc<Settings>` that does not track later writes. A
caller that holds one across several reads sees a consistent set, which is
what the spec's "A reader mid-frame sees one consistent set" scenario asks
for. The snapshot that caused #238 was a `Settings` held for a window's
lifetime; this one is held for a frame or a function. The distinction is
lifetime, and it is the thing to keep honest in review.

To make the old mistake hard to repeat, the window structs stop holding
`Settings` entirely. A window that wants a value reads it when it needs it.

### Writes are read-modify-swap against the current value

A writer takes the current `Arc<Settings>`, clones it, applies its change, and
swaps. It never starts from a value it captured earlier. That is what makes
"A write preserves values written elsewhere" hold without any caller having to
know about the hazard - the four hand-written `Settings::load()` guards exist
today precisely because that obligation was on the caller.

Persisting stays per-document: a preference write calls `persist_preferences`,
an import writes the documents it touched. The whole-surface `persist()` stays
available for the migration path, which is the only caller with a legitimate
claim to it.

### The refresh machinery is deleted, not adapted

`settings_broadcast`, `Settings::reload_preferences`, its `#[serde(skip)]`
transplant, the transplant's in-step test, and the four hand-written reloads
all exist to keep copies honest. With no copies they have nothing to do.
Deleting them is most of this change's value: the transplant in particular
silently clears any collection added to the struct and forgotten in it.

## Risks / Trade-offs

- **A reader holds an `Arc<Settings>` across an await or a long operation and
  acts on stale values.** → The old bug in miniature, at a much shorter
  lifetime. Mitigation: no struct field holds an `Arc<Settings>`; it is a
  local. A size-check-style review rule is cheaper than a lint here, and the
  window structs no longer having a `settings` field makes the wrong shape
  visibly unusual.
- **Two writers race: both clone the current value, both swap, and the second
  swap drops the first's change.** → This is the lost update moved, not
  removed. The window is microseconds rather than a window's lifetime, and
  GPUI writes come from one thread, so the race is not reachable today.
  Mitigation: the swap is a `rcu`-style compare-and-retry rather than a bare
  `store`, which closes it outright and costs nothing while uncontended.
- **Clone cost on write grows with the roster.** → A large roster makes every
  preference write clone every saved agent. Mitigation: measure before
  optimizing; if it bites, the collections move behind their own `Arc`s so the
  clone is a refcount bump. Not worth doing pre-emptively.
- **106 read sites is a large mechanical diff.** → Reviewability suffers and a
  wrong conversion hides in the noise. Mitigation: the migration plan below
  splits it so no PR mixes mechanical conversion with behavioral change.

## Migration Plan

Three stages, each landing on its own and leaving the app working:

1. Introduce the shared handle in `knot-core` alongside the existing owned
   `Settings`, and register the global in `knot`. Nothing reads it yet.
2. Move readers and writers over, one owner struct at a time, deleting each
   struct's `settings` field as its last reader goes. The two `knot-core`
   import paths move in this stage, since they are the live write-side defect.
3. Delete `settings_broadcast`, `reload_preferences`, the transplant test and
   the four hand-written reloads once nothing references them.

Rollback is per-stage: stage 1 is additive, stages 2 and 3 revert
independently.

## Open Questions

- Whether the settings global should be readable without a `&App` for the
  benefit of `knot-core`'s own tests, or whether those tests construct a
  handle directly. Either works; it does not change the specs or the task
  breakdown, and the answer will be obvious once stage 1 is written.
