# Proposal

## Why

Typing into a shell agent's terminal pane is visibly laggy. Every frame that
pane draws asks the platform text system to enumerate every installed font
family - twice - and on macOS that call goes through
`CTFontCollectionCreateMatchingFontDescriptors`, measured at 11.4ms per call
on a machine with 181 families. The terminal repaints on each keystroke echo,
so roughly 23ms of the main thread is spent on font enumeration before
anything is drawn, against a 16ms frame budget. Panel-mode agents take their
font from the theme and are unaffected, which is why only a shell agent's
terminal feels slow.

This is the same "no I/O on the render path" defect the project has already
recorded three times (`.claude/rules/rust-structure.md`); this is the fourth
place it appears, and the first where the cost is a per-keystroke one.

## What Changes

- `terminal_font_family` stops enumerating the installed font families on
  every call. The resolved family is remembered against the name that was
  asked for, and the enumeration runs only when that name has not been
  resolved yet.
- The cache is invalidated when the terminal font setting changes, so an
  Appearance-tab edit still takes effect on the next frame.
- The fallback behaviour is unchanged: a name the text system cannot resolve
  still falls back to the embedded "JetBrains Mono" rather than to the
  proportional UI font.
- A regression test pins the contract - resolving the same family repeatedly
  enumerates once - so a future caller cannot quietly put the enumeration
  back on the frame.

Not in scope, though both were found while diagnosing this and are worth
their own changes:

- `settings_window/controls.rs` enumerates the font families once per row it
  renders. The settings window does not redraw per keystroke, so it is not
  this defect.
- `knot_terminal::Grid::is_selected` recomputes the selection range for every
  cell, making a drag-selection O(cells) range computations per frame.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `terminal-rendering`: adds a requirement that drawing or sizing a terminal
  frame does not enumerate the system's installed fonts, so per-frame cost
  stays independent of how many fonts are installed.

## Impact

- `crates/knot/src/workspace_window/chrome.rs` - `terminal_font_family`, the
  only caller of `all_font_names` outside the settings window.
- `crates/knot/src/workspace_window/render/mod.rs`,
  `render/content.rs`, `terminal_input.rs` - the four call sites, which move
  to the cached lookup.
- `crates/knot/src/workspace_window/settings_refresh.rs` - invalidation when
  the terminal font setting changes.
- No new dependencies. No change to persisted settings or to any spec other
  than `terminal-rendering`.
