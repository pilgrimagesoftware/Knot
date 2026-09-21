# Proposal

## Why

"About Knot" opens a modal alert dialog carrying one hardcoded English
sentence and an OK button. It names no version, no build, and no credits, so
it cannot answer the question an About box exists to answer - "which Knot am
I running, and who made it?" - which is exactly what a user needs before
filing a bug. Being modal, it also blocks the window it opened over, which no
About box on any desktop platform does.

## What Changes

- **BREAKING** (behavioral, no API): About Knot stops opening an alert dialog
  over a workspace window and instead opens its own window. The window is
  non-modal: the rest of the app keeps working while it is open.
- The window is a singleton. Choosing About Knot again while it is open
  brings the existing window forward rather than opening a second one, the
  same contract the settings window already has.
- The window shows, on macOS, the standard shape of a macOS About box, in
  this order: the app icon, the app name, the version and build, a copyright
  line, and a credits area.
- Version and build are real values, not prose: the version is the `knot`
  crate's version, and the build identifier is stamped at compile time from
  the git commit and build date, with a defined fallback for a build made
  outside a git checkout (a source tarball).
- Credits name the author, the license, and the third-party work the app
  embeds and is obliged to attribute - the SIL OFL fonts shipped in the
  binary, and the toolkit and terminal engine it is built on.
- The version/build line is selectable or copyable, so a user can paste it
  into a bug report rather than transcribe it.
- The window closes with the platform-standard gesture: the close button, and
  `Escape`. It is not resizable and not minimizable - it holds a fixed amount
  of text.
- On non-macOS platforms the same content is shown in the idiom of that
  platform: a titled "About Knot" window carrying an explicit Close button,
  since those platforms do not expect a dialog dismissed only by its window
  chrome.

Non-goals, deliberately out of scope:

- No "Check for Updates" control. Nothing in the Rust port updates itself
  yet, and `settings-ui` already forbids such a control appearing before it
  does.
- No full license text viewer, acknowledgements browser, or release notes.
  The credits area names what is embedded and under which license; it does
  not reproduce the licenses.
- No change to the Swift reference app, which has no About window to port
  from - this behavior is specified for the Rust port directly.

## Capabilities

### New Capabilities
- `about-ui`: the About window - how it is opened, that it is non-modal and
  single-instance, what identity, version, build and credit information it
  shows, and how it is dismissed on each platform.

### Modified Capabilities
(none - `app-menu` describes the Agents menu only, and `settings-ui` owns the
settings window's own open/singleton contract; the About item's contract goes
with the window it opens.)

## Impact

- `crates/knot`: a new `about_window` module rendering the window; the
  `AboutKnot` action handler in `app_bootstrap.rs` stops opening an alert
  dialog and opens/raises that window instead, holding its handle the way
  `OpenSettings` already holds the settings window's; `window_options.rs`
  gains the window's options.
- `crates/knot`: a `build.rs` stamping the commit and build date into the
  binary as compile-time environment values.
- `crates/knot-core`: new localized strings for every line the window shows.
- Bundled assets: the About window needs an app icon larger than the existing
  32px title-bar glyph; the packaging icon already in the crate covers it.
- The existing About test moves from asserting "a dialog opened on the active
  window" to asserting the window opens once and is raised rather than
  duplicated.
