# Design

## Context

See proposal.md — Why. The mechanics that shape the approach:

- `WorkspaceWindow::sidebar_title_bar` (`crates/knot/src/workspace_window/render/mod.rs:143`)
  is an associated function taking `(compact, cx)`. It has no access to the
  window's identity, which is why it can only draw a constant.
- The window already holds `store: Arc<Mutex<AgentStore>>` and `workspace_id`,
  and already locks that store once per frame for `agent_row_snapshot`. The
  workspace's name is one lookup away on a lock the frame already takes.
- The window also holds its own `window_handle`, and `render` receives
  `window: &mut Window`, so it can set its own OS title at any point.
- `open.rs:70` sets the OS title once, at open, from a name read at open.
- The rename path (`workspace_manager::save_name`) mutates the shared store,
  persists, and calls `cx.notify()` — which schedules a repaint of the
  Workspaces window only.

That last point is the whole difficulty. Reading live shared state does not
cause a paint in GPUI: `cx.notify()` schedules the calling window, and every
other window keeps showing the frame it last drew. `crates/knot/src/import_window/window.rs:164`
is the existing precedent for the fix.

## Goals / Non-Goals

**Goals:**

- One source for the displayed name — the shared store — so no second copy can
  go stale.
- Keep the OS title and the in-app title bar in agreement by construction,
  rather than by two call sites that happen to pass the same string.

**Non-Goals:**

- A general cross-window change-notification mechanism. `refresh_windows()` is
  a blunt instrument and is the right size for this change; a targeted
  subscription is a larger design that this change does not need and should not
  pre-empt.
- Touching how the window is opened or how its bounds are restored.

## Decisions

### Read the name per frame from the store, do not cache it on the window

`sidebar_title_bar` becomes a method and looks the workspace up by
`workspace_id` on the store lock the frame already holds.

The alternative — a `workspace_name: String` field set at open — is what issue
#238 is filed about for `settings`: a snapshot on `WorkspaceWindow` that the
rest of the app cannot see, so edits made elsewhere do not apply and the next
write from this window reverts them. A name is smaller than a settings struct
but fails the same way, and it would need its own invalidation path. A lookup
on an already-held lock costs nothing and cannot drift.

A missing workspace needs no fallback string: the render path for a deleted
workspace returns early with an empty title bar and `workspace.missing`
(`render/mod.rs:250`), above where the sidebar is built.

### Propagate a rename with `cx.refresh_windows()` from the rename path

After a successful `rename_workspace`, `save_name` calls `cx.refresh_windows()`
in addition to `cx.notify()`. Every open workspace window repaints, re-reads
the store, and the renamed one draws its new name; the others redraw
identically, which is what "leaves other windows alone" means observably.

Alternatives considered:

- *The manager sets the other window's OS title directly.* It has no handle to
  it — handles live on the windows themselves — so this means building a
  registry of open workspace windows for one string. Rejected as disproportionate.
- *A polling repaint.* `spawn_repaint_poll` already runs for each workspace
  window, so a rename would eventually show. Rejected: "eventually" is a visible
  lag on a direct user action, and a poll that exists for PTY and session events
  is the wrong thing to hang UI correctness on.

### Set the OS title from render, guarded by the last title set

The window keeps a `titled_as: String` recording the name it last passed to
`set_window_title`. Each render compares the name it just read; on a difference
it calls `window.set_window_title` and stores it.

This is a cache of an *output*, not of the state — it can only cause a redundant
title write if it drifts, never a wrong title bar — so it does not carry the
#238 hazard the rejected `workspace_name` field would. The guard exists because
`set_window_title` crosses into AppKit and should not run on every frame of a
sidebar drag.

The alternative, setting the title only at open and again at rename, needs the
rename path to reach the window and lands back at the registry problem above.

### Compact keeps the icon and moves the name into a tooltip

The compact bullet in `agent-list-ui` already said to hide the label. What
changes is that the hidden label now carries information rather than repeating
the app's name, so the compact layout follows what the compact agent row and the
new-agent button already do: hide the text, offer it as a tooltip
(`render/sidebar.rs:73`, `render/mod.rs:172`). Without it, a narrow sidebar is a
window with no visible identity at all.

## Risks / Trade-offs

- **`refresh_windows()` repaints every window in the app, not just workspace
  windows** → It is called once per confirmed rename — a deliberate, infrequent
  user action — not on a timer or a drag. The import window already does the
  same thing on a comparable event.
- **The title bar now reads the store on the render path** → It is a
  `parking_lot::Mutex` lock the frame already takes for the agent rows, and a
  `Vec` scan over workspaces, which are few. No I/O, so the rule that
  `diff_stats.rs` exists to enforce is not in play. Take the name from the same
  lock scope as the row snapshot rather than locking a second time.
- **A workspace name is user data rendered as chrome** → It is already rendered
  as the OS window title, so this exposes nothing new. It stays on one line with
  an ellipsis, and `single_line()` (`app_support.rs:261`) is what keeps an
  embedded newline from turning the title bar into two rows — the helper exists
  because `whitespace_nowrap` alone does not do it.
- **`app.name` loses its use here** → It is still used by the About window and
  `set_process_name`, so the key stays and the l10n catalog test
  (`crates/knot/src/tests/l10n_catalog.rs`) needs no change.
