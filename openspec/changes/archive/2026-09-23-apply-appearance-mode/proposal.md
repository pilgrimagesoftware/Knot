# Proposal

## Why

The General pane's Appearance picker is inert. It writes
`Settings::appearance_mode`, persists it, and reads it back on launch to draw
its own label — and nothing else in the workspace reads the field.
`grep -rn appearance_mode crates` finds the struct field, its default, the
picker that writes it, and the label that reads it back. That is the whole set.

The theme is chosen entirely from the OS. `app_bootstrap.rs` calls
`Theme::change(cx.window_appearance(), None, cx)` once at startup, and
`app_support::observe_system_appearance` re-runs `Theme::change` with the OS
appearance on every flip, for every window. A user who picks Light on a Mac in
Dark mode gets a picker that says "Light" and an app that stays dark, and a
relaunch restores the same inert choice.

The Swift reference applies the mode on write (`AppSettings.swift:143`,
`didSet { applyAppearance() }`): `light`/`dark` force `NSApp.appearance`,
`system` follows the OS, `auto` derives from the terminal background colour's
luminance.

## What Changes

- A chosen appearance becomes the app's appearance. `Light` and `Dark` force
  the theme regardless of what the OS reports, at startup and for every window
  opened afterwards.
- An OS appearance flip no longer overwrites an explicit choice.
  `observe_system_appearance` resolves the effective appearance through the
  stored mode instead of passing `cx.window_appearance()` straight to
  `Theme::change`, so a Mac switching to Dark at sunset leaves a user who
  picked Light in light.
- Picking a mode repaints immediately, in every open window. Today the picker
  persists and nothing redraws; the setting must not need a relaunch to be
  visible.
- The Appearance hint stops describing behaviour the port does not have. It
  currently reads "Derives color scheme from terminal background color",
  carried over from the reference.

### Non-goals

- Deriving `Auto` from a terminal background colour. The port has no terminal
  background setting to read a luminance from — `grep` for one in
  `knot-core/src/settings/store.rs` finds nothing — so there is no signal to
  derive from. `Auto` follows the OS, exactly as `System` does, and the spec
  says so rather than leaving the two silently identical. Making `Auto`
  meaningful is a follow-up to whichever change gives the port a configurable
  terminal background.
- Removing the `Auto` variant. It is `APPEARANCE_MODE_DEFAULT` and is already
  written into users' settings files; dropping it would change the meaning of
  a stored document on upgrade.
- Per-window appearance. The mode is one app-wide choice, as in the reference.
- Branching window chrome colours on the mode
  (`effectiveBackgroundColor`/`sidebarBackgroundColor` in the reference). The
  port derives chrome from the system palette in `apply_system_palette`, and
  reworking that is a separate concern from making the picker do anything at
  all.

## Capabilities

### New Capabilities

- `appearance-mode` — how the stored appearance preference resolves to a
  painted appearance, and when it is re-resolved.

### Modified Capabilities

- `settings-ui` — the Appearance control applies its selection, not merely
  persists it.
