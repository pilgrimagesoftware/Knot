# Design

## Context

See `proposal.md` - Why. The constraints that shape the approach:

- `about_knot` in `app_bootstrap.rs` currently defers a `window.update` onto
  the active window and calls `open_alert_dialog`. The defer exists because a
  menu dispatch already runs inside that window's update; a re-entrant
  `window.update` is refused by gpui.
- `OpenSettings` already solves the same shape of problem - a singleton
  auxiliary window opened from the application menu - by holding an
  `Rc<RefCell<Option<AnyWindowHandle>>>` in `run()` and calling
  `cx.open_window` from inside the action closure. It does not need the
  defer: opening a *new* window from an action is not re-entrant.
- Nothing in the crate currently knows its own version or commit. There is no
  `build.rs` anywhere in the workspace.
- `assets/app-icon-32.png` is a 32px title-bar glyph; `assets/icon/icon.png`
  (the packaging icon) is full resolution.
- The workspace pins `time` with `formatting` in `[workspace.dependencies]`.
- `Escape` is not handled anywhere in the app today.

## Goals / Non-Goals

**Goals:**

- Reuse the settings window's open/raise/singleton mechanics rather than
  inventing a second pattern for auxiliary windows.
- Make version and build derive from the build itself, so they cannot go
  stale.
- Keep the platform difference (macOS vs. elsewhere) to the smallest possible
  surface: one `cfg`-gated element, not two window implementations.

**Non-Goals:**

- No shared "auxiliary window" abstraction extracted from settings and
  About. Two instances of a pattern is not yet a pattern; extracting one now
  would be speculative.
- No change to how `OpenSettings` works.
- No reproducible-build guarantee. The build date is stamped when `build.rs`
  runs.

## Decisions

### The handle lives where the settings handle lives

`run()` gains a second `Rc<RefCell<Option<AnyWindowHandle>>>` and registers
`AboutKnot` as a closure over it, exactly as `OpenSettings` is registered.
`open_about_window(handle, cx)` tries `existing.update(.., window.activate_window())`
first and opens a new window only when that fails - the same "a closed window
fails its update" test the settings path relies on, which means no
close-observer bookkeeping is needed to clear the handle.

*Alternative considered:* keeping `about_knot` a free function with a
`thread_local!` handle. Rejected: it hides the lifetime of the handle from
`run()`, where every other window's state is visible.

*Consequence:* `about_knot` stops being a free function, so the existing
regression test, which registers that function directly, is rewritten around
the closure. The re-entrancy hazard that test guards against disappears with
the alert dialog - the new test asserts the window contract (opens once,
raises rather than duplicates) instead.

### Version and build come from a `build.rs`

`crates/knot/build.rs` emits two `cargo:rustc-env` values read at compile
time with `env!`:

- `KNOT_BUILD_COMMIT` - `git rev-parse --short=12 HEAD`, with a `-dirty`
  suffix when `git status --porcelain` is non-empty. When `git` is absent or
  fails (a source tarball), the literal `unknown`.
- `KNOT_BUILD_DATE` - today's UTC date, `YYYY-MM-DD`, via the workspace `time`
  crate as a `[build-dependencies]` entry.

The version is `env!("CARGO_PKG_VERSION")` - the `knot` crate's version, which
is what the release workflow bumps.

`build.rs` emits `cargo:rerun-if-changed` for `.git/HEAD` and the ref it
points at, so a commit or branch switch restamps and an ordinary rebuild does
not.

*Alternative considered:* a `vergen`-style crate. Rejected: ~20 lines of
`Command` plus one build-dependency already in the workspace beats a new
third-party build-time dependency for two strings.

*Alternative considered:* commit date instead of build date. Rejected: it is
unavailable in exactly the tarball case where a date is still wanted.

### One view, one `cfg`-gated affordance

`crates/knot/src/about_window/mod.rs` holds `open_about_window` and an
`AboutWindow` view rendering, top to bottom: icon, app name, version/build
line, copyright, credits. The only platform difference in the tree is a
`#[cfg(not(target_os = "macos"))]` Close button appended at the bottom -
`Escape` and the close button work identically on both.

`Escape` is handled by tracking a `FocusHandle` on the view's root element,
focusing it when the window opens, and matching `event.keystroke.key` in an
`on_key_down` listener, calling `window.remove_window()`. A global key binding
was rejected: it would bind `Escape` app-wide, where other windows may want it.

The window is opened with `is_resizable: false`, `is_minimizable: false`, a
fixed `window_bounds`, and a titlebar titled from the catalog. On macOS the
title text sits above content that names the app anyway; this is what the
platform does with an About box that uses a standard titlebar, and it avoids
a custom-drawn titlebar for one window.

### The icon is embedded at build time, downscaled at render

`assets/icon/icon.png` is `include_bytes!`-embedded and rendered at a fixed
size, as `app_titlebar_icon` already does with the 32px asset. No second
asset file is added.

*Trade-off:* the packaging icon is ~368 KB and is embedded in the binary in
addition to being used by the packager. Adding a third icon size to keep the
binary smaller trades a real asset-maintenance cost against a fraction of a
megabyte; not worth it.

### Copying the build details

The version/build line carries a copy affordance that writes
`"Knot <version> (<build>)"` to the clipboard via
`cx.write_to_clipboard(ClipboardItem::new_string(..))`, the same call the
settings window's MCP URL copy uses. A copy action is chosen over making the
text selectable because the toolkit's selectable text is an input control,
which would read as an editable field in an About box.

### Strings

Every label goes under an `about:` block in `crates/knot-core/locales/en.yml`.
The credits body is a small set of keys (author, license, and one per
attributed work) rather than one long blob, so a translator sees sentences
rather than a page. Project names and license identifiers are interpolated,
not translated.

## Risks / Trade-offs

- [`build.rs` shells out to `git`, which is not guaranteed on a build
  machine] → The failure path is already the specified tarball path:
  `unknown`. `build.rs` never fails the build on a missing or failing `git`.
- [`rerun-if-changed` on `.git/HEAD` misses the case where `HEAD` is
  unchanged but the working tree became dirty, so `-dirty` can be stale] →
  Accepted. The dirty marker is a courtesy for local builds; released builds
  are made from clean checkouts in CI.
- [The About window is the first window in the app to handle `Escape`, so
  there is no established pattern to copy] → Scoped to this view's own focus
  handle, so it cannot affect key handling in any other window.
- [Embedding the full-resolution packaging icon grows the binary] → Bounded
  and measured in tasks; the alternative is a third icon asset to keep in
  sync.

## Open Questions

None.
