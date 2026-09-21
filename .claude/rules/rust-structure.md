---
paths:
  - "crates/**"
---

# Structure and maintainability rules for the Rust workspace

Each of these exists because it was violated and cost something. The cost is
named so you can tell when the rule genuinely does not apply.

## File size: 700 lines, enforced

`make rust-size` fails the build above 700 lines per `.rs` file, and CI runs
it. Do not raise `RUST_FILE_LINE_LIMIT` to make a change fit.

This was applied once by hand and then regressed: `workspace_window/mod.rs`
came out of that split at 2216 lines and reached 4578 four days later, with a
single `render` function of 917 lines. Nothing enforced it, so it drifted back.

When a file approaches the limit, split it **by concern, not by line count**:

- Pull out the group of items that answer one question.
- Give the new module a doc comment saying what it owns and what it does not.
- An inherent `impl` can be split across files - `impl Foo { .. }` may appear
  in several modules of the same crate. A large `impl` therefore splits with
  no call site changing.
- Keep the parent's public surface identical by re-exporting (`pub(crate) use
  submodule::*;`). Which file an item lives in is the module's business, not
  its callers'.

Colocated `#[cfg(test)] mod tests` counts toward the limit. Move it to a
sibling `tests.rs` (`foo.rs` + `foo/tests.rs` both work) before splitting
production code that is fine as it is.

## Visibility: widen only as far as the move requires

When splitting, a private item used by a sibling becomes `pub(super)`, or
`pub(in crate::some::module)` when the module nests deeper. It does not become
`pub(crate)` "to be safe", and it never becomes `pub`.

A visibility wider than the code needs is a claim about who may depend on the
item, and it is the claim - not the keyword - that costs later.

## No crate-wide `allow`

`crates/knot/src/main.rs` carried `#![allow(dead_code)]`. It was hiding 24
items, including the entire decision layer of a **promoted spec that is not
implemented** (`desktop-notifications`, issue #222) - passing tests, green CI,
and no notification ever raised.

If something must be allowed, allow it on the item, with a comment saying why:

```rust
// UNWIRED(#222): desktop-notifications' decision layer. Nothing calls
// `show_system_notification`, so this is reached only from tests.
#[allow(dead_code)]
pub(crate) fn should_notify(..) -> bool { .. }
```

Markers in use, both greppable:

- `UNWIRED` - ported from the Swift reference, no caller yet. Reference the
  tracking issue when there is one.
- `SUPERSEDED` - a newer path replaced this; it is waiting to be deleted.

A test that exercises unwired code is not coverage. Say so in the test
module's doc comment, as `tests/notifications.rs` does.

## Constants live in `consts.rs`

One per crate. A value belongs there when it is a *decision* - how often to
poll, how stale a cache may get, what colour a state is. A value stays inline
when it is part of one element's layout: a padding, a gap, a single width.

Constants that are only meaningful together at one call site may stay local,
named and documented in place; hoisting those makes them harder to find, not
easier.

## No I/O on the render path

GPUI re-renders on every keystroke. A subprocess, a file read or a blocking
lock inside `Render::render` runs at that rate.

`workspace_window` learned this twice: `git diff --numstat` ran inline in the
header, was moved behind a TTL cache and `spawn_blocking`, and then came back
inline in the dashboard - once per card, per frame. Route through the cache
that already exists.

## `Mutex` poisoning: one policy per crate

`workspace_window/mod.rs` had 22 `lock().unwrap()` against 52 `if let Ok(..)`,
sometimes two lines apart. Whether a poisoned lock crashes the app or is
silently ignored depended on which call site happened to fire.

Prefer the degrading form on the UI thread - a window that must keep rendering
should not panic because a background thread died. `knot-terminal/src/pty.rs`
shows the better shape still: map poisoning to a typed error.

## Owning spawned work

A `tokio::spawn` whose `JoinHandle` is dropped cannot be cancelled and
swallows its panic. Store the handle and abort it in `Drop`, as
`knot-mcp/src/server.rs` and `knot-watch/src/lib.rs` do.

A background task must never hold a strong `Arc` back to the thing that owns
its lifetime. `Transport`'s stdout pump held an `Arc<Transport>`, and
`Transport` owns the `kill_on_drop` child - so the task waited for a pipe that
only closed when the child died, and the child only died when the task let go.
That cycle orphaned one adapter subprocess per panel agent. Use a `Weak` and
upgrade per iteration.

## Parallel per-key maps

`WorkspaceWindow` holds a dozen `BTreeMap<Uuid, _>` that must all be pruned
together; its own `teardown_session` doc comment admits the hazard, and three
of them were missed anyway and grew without bound.

Prefer one struct per key over N maps keyed alike. Where the maps already
exist, teardown belongs in one function and every new field must be added to
it in the same commit.

## Orphaned files

Rust emits no diagnostic for a `.rs` file that no `mod` declares - it is
simply not compiled. `knot-mcp-tools` carried three such files for four days,
duplicating live code and already drifting from it.

After splitting a module, confirm every new file is reachable: `cargo check`
passing is not evidence, because an unreferenced file cannot fail to compile.

## Enums over strings for closed vocabularies

`appearance_mode`, `ai_provider`, `autopilot_action`, `agent_type` are stored
as `String` and matched in several places each with a `_ => default` arm, so a
corrupt value is indistinguishable from the real default (issue #224). New
closed vocabularies get an enum with `Display`/`FromStr`, serialized as the
same string, so every match is exhaustive and the compiler finds the next site.

## User-facing text

Goes through `knot_core::l10n::t` with a key in
`crates/knot-core/locales/en.yml`. Much existing code does not (issue #223);
match `about_window` and `agent_menu`, not `settings_window`.

Tests assert the catalogue **key resolves**, never the English copy - a copy
edit should not fail a test.
