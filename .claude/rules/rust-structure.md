---
paths:
  - "crates/**"
---

# Structure and maintainability rules for the Rust workspace

Each of these exists because it was violated and cost something. The cost is
named so you can tell when the rule genuinely does not apply.

## File size: 700 lines, enforced

`make size-check` fails the build above 700 lines per `.rs` file, and CI runs
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

`git diff --numstat` has been on this path three times: the workspace
window's header, then its dashboard once per card, then the command center -
which shows *every* workspace's agents, so it was the worst of the three and
the last to be found.

The cache that answers this is `knot/src/diff_stats.rs`. A render asks it for
what it already has (`snapshot`) and separately asks it to refresh what has
aged out (`claim_refresh`); the `git` call runs off the main thread and a
later frame draws the answer. Route through it rather than adding a fourth
instance.

## Off-thread results must reach a frame

The rule above gets work off the render path. This one is about the answer
coming back, which is a separate problem and the one that keeps recurring:
six defects in one day, all of the shape "the code ran, the state was
correct, and the result never reached the screen".

Work that finishes off the main thread reports itself through a flag -
`RefreshCache`'s dirty bit, `PanelSessionHandle::take_dirty`, an
`Arc<AtomicBool>` a `spawn_blocking` sets. `repaint_poll_tick` reads those
flags and calls `cx.notify()`. Every link in that chain has now been broken
at least once:

- **The flag nobody polls.** `pull_request_states` set its dirty bit on every
  landed fetch and nothing ever called `take_changed()` on it - the poll read
  `diff_stats` alone. Rows drew when something unrelated happened to notify,
  and on a workspace with nothing running, possibly never.
- **The flag read and then discarded.** `panel_needs_repaint` took
  `diff_stats.take_changed()` - which *clears* as it reads - and then hit
  `let Some(slot) = .. else { return false; }`. For every Terminal-mode agent
  the answer was consumed and thrown away.
- **The work reported to nobody.** `sync_panel_agent_states` wrote every panel
  session's lifecycle into the store and returned `()`. An unselected agent
  finishing its turn reached the store and stopped there; its sidebar dot was
  on screen and did not move.
- **The claim that never records.** `claim_refresh` *marks* the key as
  requested before handing back the writer. Drop that writer without calling
  `record()` and the key reads as fresh for the whole `MAX_AGE` with nothing
  behind it - not a missed repaint but a stall, which looks like a slow
  subprocess rather than a bug.
- **The write that rode on someone else's notify.** `display-markdown` and
  `view-mermaid` write the agent store from the MCP server's thread, which has
  no context to notify from, and nothing in the poll's chain read the fields
  they write. It looked correct for a year because an agent calling the tool
  is usually mid-turn, so its own streaming repainted the window and the pane
  arrived with it. The case with no cover is an artifact set for an agent that
  is *not* streaming - a Terminal-mode agent, an idle one, or one a different
  agent named by `agentId`.

That last one generalises the rule, so state it plainly: **the hazard does not
arrive with newly-async code.** A write that was always asynchronous and always
rode on an incidental repaint has the same defect, and looks fine until the
masking case stops applying. When auditing, do not ask "what did I just make
async" - ask "what reads this, and what repaints when it changes". Every
instance found so far has had a real write, a real reader, and something else's
notify standing in for the missing one.

What to check, in the order the mistakes were actually made:

1. A new flag, cache or `spawn_blocking` result must appear in
   `repaint_poll_tick`'s `if` chain. A cache nobody polls is
   indistinguishable from one that works until the window goes quiet.
2. A clearing read must not be *reachable on a path that discards it*. The
   safe shape puts it where it cannot run on that path - inside the option
   chain or the match guard, as `grid.lock().take_dirty()` and
   `if handle.take_dirty()` both do - not merely below the early return.
3. Assign clearing reads to locals before combining them. `a.take_changed()
   || b.take_changed()` skips the second whenever the first is true, and
   because the read is a consuming swap, the skipped flag stays set and fires
   a spurious repaint on the next tick.
4. Between `claim_refresh` and the writer moving into the work there must be
   no early return and no `?`. Hand it to `spawn_blocking` on the next line,
   which is safe by construction rather than by inspection.

None of these fail a test or a lint. The symptom is an absence, and an
absence has no output channel - which is why they are worth checking by hand
when the chain is touched.

## Locks: `parking_lot`, and a guard that does not outlive its statement

`workspace_window/mod.rs` once had 22 `lock().unwrap()` against 52
`if let Ok(..)`, sometimes two lines apart, so whether a poisoned lock crashed
the app or was silently ignored depended on which call site happened to fire.
The workspace answered that by deleting the question: locks are
`parking_lot::Mutex`, which has no poisoning and whose `lock()` returns the
guard directly. Do not write `if let Ok(..)` or `.unwrap()` around one - there
is no `Result` to unwrap.

`tokio::sync::Mutex` stays in exactly one place, `knot-acp`'s transport, where
the guard is held across an `.await`. Everywhere else the `parking_lot` guard
is `!Send`, which is what makes the compiler reject that mistake.

What still needs care is *how long* a guard lives. Scope it so it is dropped
before any blocking call - `persist_agents` takes the store lock, copies what
it needs, drops it, and only then writes the settings file - and take it once
around a loop rather than once per iteration, or another window can observe a
workspace half restarted.

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

## `mod.rs` declares; it does not implement

A `mod.rs` holds module declarations, re-exports, and the doc comment saying
what the module owns. Implementation goes in sibling files named for what they
do - `import_window/window.rs` for the entity and its lifecycle,
`import_window/pane.rs` for what it draws.

Unlike the rest of these rules, this one is a standing decision rather than a
postmortem: it was adopted while moving the Import surface out of the settings
window, before it had cost anything. The reasoning is that the 700-line limit
is a ceiling, not a target, and it is the only signal that had been enforcing
placement at all. `import_window` would have been 443 lines in one `mod.rs` -
comfortably legal, and still wrong, because nothing about the name says where
the rendering is.

Six `mod.rs` files predated the rule - `agent_editor`, `panel_state`,
`panel_view`, `settings_window`, `about_window` and `workspace_window`. They
were treated as a backlog rather than as exceptions, and split under issue
#300, one module per PR so a regression is bisectable to a single split.
Grandfathering has to be argued for, not assumed, or a rule the codebase
visibly breaks stops being a rule.

Splitting those six turned up one hazard worth naming. A parent's private
items are visible to its descendants, so anything declared in a `mod.rs` can be
read from every child module. Moving it to a sibling breaks that: widen to
`pub(super)` - the module and no further - never `pub(crate)` "to be safe".
`rustc` reporting a `pub(crate) use` re-export as an unused import is the
signal that nothing outside actually consumed the item, and that it should
narrow rather than be re-exported.

`mod.rs` files still holding implementation elsewhere in the workspace -
`workspace_window/render`, `workspace_manager` and `plan_view` in `knot`, and
several in `knot-acp` and `knot-terminal` - were outside that issue's scope and
are still to do.

A `mod.rs` that is only declarations and re-exports also makes the module's
shape readable in one screen, which is the same argument as splitting by
concern: you should be able to see what a module is made of without reading
what it does.

## Orphaned files

Rust emits no diagnostic for a `.rs` file that no `mod` declares - it is
simply not compiled. `knot-mcp-tools` carried three such files for four days,
duplicating live code and already drifting from it.

After splitting a module, confirm every new file is reachable: `cargo check`
passing is not evidence, because an unreferenced file cannot fail to compile.

## Vocabularies: an enum when closed, one roster when open

`appearance_mode`, `ai_provider` and `autopilot_action` were `String`s matched
in several places each with a `_ => default` arm, so a corrupt value was
indistinguishable from the real default (issue #224). They are enums now, in
`knot-core/src/settings/vocabulary.rs`, serialized as the same strings they
have always been stored as, with an unrecognized value turned into the default
*once*, at load, where it can be seen.

`agent_type` is the other case and stays a `String`: MCP `agent_create` takes
whatever an agent asks for, and a type with no ACP adapter deliberately
launches through the terminal path, so an unknown value is a working
configuration. What it gets instead is one roster -
`knot_core::agent_type::ALL` - carrying the label and the flags every crate
was deciding by hand, with the pickers built by filtering it. Per-type data
that cannot live there (the adapter table, the icons, the install commands)
keeps a test that fails when a roster row has nothing matching it.

So: an enum when the set is genuinely closed, a roster plus coverage tests
when it is not. What is not acceptable is the same list written out in six
places.

## User-facing text

Goes through `knot_core::l10n::t` with a key in
`crates/knot-core/locales/en.yml` (issue #223, now closed - every window's
chrome is routed, so match any of them).

A sentence that embeds a value stays **one** entry and substitutes through
`t_with`; assembling it from fragments at the call site takes the word order
away from the translator. `pluralize` does the same for a count and its noun.

Tests assert the catalogue **key resolves**, never the English copy - a copy
edit should not fail a test. For a sentence with a placeholder, assert the
value survives substitution too: a body that lost its `%{name}` still resolves,
and still asks "Restart ?".

Three times in #223 the reason a file was unlocalized was a helper whose
signature took `&'static str`. If text will not go through the catalog, check
the signature before concluding the call site is special.
