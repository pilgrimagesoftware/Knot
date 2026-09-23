# Design

## The shape chosen, and the one not chosen

Issue #238 weighs two fixes. One shares a single settings value across every
window (a GPUI global, or `Arc<Mutex<Settings>>`), removing the snapshot and
with it both the stale read and the lost update. The other keeps per-window
copies and refreshes them.

This change takes the second, deliberately, and does not claim it is the
better end state. The first is a change to every window's ownership of its
configuration and every path that writes it - roughly forty call sites - and
it is the kind of change that wants its own review rather than riding along
with a panel bug. The observable defect is that a toggle does nothing; a
refresh fixes that completely, and does not foreclose the shared store later.

## Why the preferences document alone

`Settings` is one surface over seven documents. The scalars live in
`preferences.json`; the collections are `#[serde(skip)]` and each has its own
file. A refresh that reloaded everything would pull the roster back to what
was last written to disk, which is the same class of bug as the write-side
lost update - a window whose roster is ahead of the file would be silently
rewound.

So the refresh decodes only the preferences document and transplants the
collections from the value being refreshed. The set of fields to preserve is
exactly the `#[serde(skip)]` set, which is also the set `read_documents` fills
from its own files; a test asserts the two stay in step, because a collection
added to one and forgotten in the other would be silently cleared on the next
preference change.

This also means a new scalar needs no work here: it is picked up by the same
`serde` decode that loads it at startup.

## Why the registry, not an observer

The window registry already holds a weak `Entity<WorkspaceWindow>` per open
workspace window, and already treats a failed update as proof the window
closed. That is exactly the liveness question a settings broadcast has to ask,
already answered in one place. A new GPUI global with observers would be a
second registry of open windows to keep correct.

## Where the I/O happens

On the settings write, in the settings window's persist path - an action the
user takes by hand, not a paint. `.claude/rules/rust-structure.md` bans I/O on
the render path, and `crates/knot/src/diff_stats.rs` exists because that rule
was broken three times. One decode of one small document per open window per
toggle is not on any hot path.
