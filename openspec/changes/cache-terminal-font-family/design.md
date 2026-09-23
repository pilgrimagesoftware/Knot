# Design

## Context

See proposal.md - Why for the measurement and the symptom.

Three constraints shape the approach:

- `chrome.rs` documents itself as holding helpers that touch no
  `WorkspaceWindow` state. `terminal_font_family` lives there today and
  reaches the text system through `cx`, which is how the cost got in.
- `grid_position` takes `&self`, and its callers hold an immutable borrow of
  `self` (the session map) across the call. Anything that needs `&mut self`
  at that point forces those callers to clone the session `Arc` first.
- The window already keeps per-frame memo fields for exactly this shape of
  problem - `panel_phases`, `last_spinner_frame`, `focused_composer` - each
  refreshed from `prepare_frame` and read by everything downstream.

## Goals / Non-Goals

**Goals:**

- Font enumeration happens when the configured name changes, not per frame.
- The four call sites read one resolved value, so a fifth caller cannot
  reintroduce the cost by copying the existing pattern.
- The resolution rule itself becomes a pure function with a unit test, the
  way `settings_window::font::font_label` already is.

**Non-Goals:**

- Reacting to fonts installed or uninstalled while the app is running. See
  Risks.
- Any change to `terminal_cell_size`. `resolve_font` is memoised inside
  gpui's text system, so its per-frame cost is a hash lookup.
- The two adjacent costs named in the proposal (settings-window rows,
  `Grid::is_selected`).

## Decisions

### Resolve once per frame in `prepare_frame`, not lazily per call

The window gets a `terminal_font` field holding the name that was asked for
and the family it resolved to. `prepare_frame` refreshes it; every call site
reads it.

Alternatives considered:

- **Lazy memo behind interior mutability** (`RefCell`/`parking_lot::Mutex` on
  the field, resolved on first call). Keeps every signature as it is, but
  puts a lock on the render path for a value only the main thread touches,
  and `.claude/rules/rust-structure.md` reserves `parking_lot::Mutex` for
  actual cross-thread state.
- **Make the call sites take `&mut self`.** Forces `grid_position`'s callers
  to restructure their borrows for no gain, and still leaves each call site
  responsible for remembering to go through the memo.

Refreshing in `prepare_frame` costs one string comparison per frame and
matches how the window already handles `focused_composer`.

### Key the memo on the requested name, not an explicit invalidation hook

Resolution re-runs when `settings.terminal_font_name` differs from the name
the memo was built for. That covers `adopt_preferences` (issue #238's
re-read) without a second code path that has to be remembered when a new way
of changing settings appears - the failure mode the settings-refresh module
exists to document.

### Split the rule out as a pure function

`chrome.rs` keeps no window state by design, so the memo cannot live there.
The resolution rule moves to a new `workspace_window/terminal_font.rs` as
`resolve(requested: &str, available: &[String]) -> SharedString` - the
signature `settings_window::font::font_label` already uses, testable without
a text system, a window, or an installed font. The memo and the
`WorkspaceWindow` accessor live beside it in the same module.

## Risks / Trade-offs

- **A font installed while Knot is running is not picked up** until the
  terminal font setting changes → The pre-existing behaviour was to notice it
  on the next frame. Accepted: installing a font mid-session and expecting an
  already-open terminal to adopt it without touching settings is not a
  workflow the app supports elsewhere, and the settings window - where a user
  would go to select the newly installed font - enumerates on its own.
- **The memo caches the fallback too**, so an unresolvable name is not
  re-checked every frame. That is the point; the spec delta pins it as
  intended rather than incidental.
- **The regression test pins a call count, not a duration.** A timing
  assertion would be flaky (`.claude/rules` and the project's no-flaky-tests
  rule both forbid that), so the test counts how many times the
  available-families list is asked for across repeated resolutions. That
  catches the defect - the enumeration returning to the frame - without
  measuring the machine.
