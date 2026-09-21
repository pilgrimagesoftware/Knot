# about-ui Specification

## Purpose
Defines the About window: how it is opened, what identity, version, build
and credit information it shows, and how it behaves and is dismissed on each
platform. The Swift reference app has no About window; this describes the
Rust port's own behavior.

## Requirements

### Requirement: About Knot opens a window, not a dialog

The application menu SHALL carry an "About Knot" item that opens the About
window. The window SHALL be non-modal: while it is open the user SHALL be
able to interact with every other window of the app - selecting agents,
typing into a panel, opening settings - without dismissing it first.

The window SHALL NOT be presented as an alert, sheet or dialog attached to
another window, and closing it SHALL leave every other window untouched.

This replaces the behavior the Rust port shipped before this change, where
About Knot opened a modal alert dialog over whichever window was active. The
Swift reference app has no About window to derive this from.

#### Scenario: The menu item opens the window

- **WHEN** the user chooses About Knot from the application menu
- **THEN** the About window opens and comes to the front

#### Scenario: The app stays usable behind it

- **WHEN** the About window is open
- **THEN** the user can click into a workspace window and keep working
  without closing the About window first

#### Scenario: No window is open

- **WHEN** the user chooses About Knot while no other window of the app is
  open
- **THEN** the About window still opens

### Requirement: One About window at a time

Choosing About Knot while the About window is already open SHALL bring the
existing window forward and focus it, rather than opening a second one. This
matches the single-window contract `settings-ui` gives the settings window.

After the window has been closed, choosing About Knot SHALL open it again.

#### Scenario: Choosing About twice

- **WHEN** the user chooses About Knot while the About window is already open
- **THEN** no second window opens, and the existing one is brought to the
  front

#### Scenario: Reopening after closing

- **WHEN** the user closes the About window and then chooses About Knot again
- **THEN** the About window opens again

### Requirement: The window identifies the running build

The About window SHALL show, as distinct pieces of information:

- The app's icon, at a size that reads as the app's identity rather than as
  a toolbar glyph.
- The app's name.
- The version of the running binary, as its released version number.
- A build identifier for the running binary, distinguishing two builds of the
  same version number from each other.
- A copyright line naming the publisher.

The version and build SHALL be the values of the binary that is running, not
text maintained by hand: a version bump SHALL be reflected without editing
the About window, and two binaries built from different commits SHALL show
different build identifiers.

When the binary is built outside a source repository - from a tarball, where
no commit is knowable - the build identifier SHALL still be shown and SHALL
state that the commit is unknown, rather than being blank, absent, or a stale
value from another build.

#### Scenario: Version and build are shown

- **WHEN** the user opens the About window
- **THEN** it shows the app icon and name, the running binary's version, a
  build identifier for it, and a copyright line

#### Scenario: A new version needs no About-window edit

- **WHEN** the app's version is bumped and the app is rebuilt
- **THEN** the About window shows the new version

#### Scenario: Two builds of one version

- **WHEN** two binaries of the same version are built from different commits
- **THEN** their About windows show different build identifiers

#### Scenario: Built outside a repository

- **WHEN** the app is built from a source tarball with no repository
  metadata available
- **THEN** the About window shows a build identifier that states the commit
  is unknown

### Requirement: The version and build can be copied

The user SHALL be able to get the version and build identifier out of the
window as text, so they can be pasted into a bug report without being
transcribed by eye.

The displayed version and build SHALL themselves be the control: clicking
them SHALL put both on the clipboard. Because text is not ordinarily
clickable, they SHALL show that they are interactive when the pointer is
over them.

#### Scenario: Copying the build details

- **WHEN** the user clicks the version and build in the About window
- **THEN** the clipboard holds them as text naming both the version and the
  build identifier

#### Scenario: The text says it can be clicked

- **WHEN** the pointer moves over the version and build
- **THEN** they are visibly distinguished from surrounding text, and the
  pointer indicates something clickable

### Requirement: The window credits what the app is built from

The About window SHALL show a credits area naming:

- The app's author.
- The license the app itself is distributed under.
- The third-party work embedded in the binary that carries an attribution
  obligation - by name and license - specifically the bundled fonts, the UI
  toolkit, and the terminal engine.
- The work this app was derived from, by name and author. The Rust app is
  its own work under its own copyright, and says so; the Swift app it was
  ported from is someone else's, and saying so is owed to them.

The credits area SHALL NOT reproduce full license texts; naming the work and
its license is what is required here. No "Check for Updates" control SHALL
appear in this window, consistent with `settings-ui` forbidding one until the
port can actually update itself.

#### Scenario: Attribution is present

- **WHEN** the user opens the About window
- **THEN** the credits area names the author, the app's license, each
  embedded third-party work with its license, and the app this one was
  derived from

#### Scenario: No update control

- **WHEN** the user opens the About window
- **THEN** it offers no control to check for or install updates

### Requirement: The window is dismissed the platform's way

The About window SHALL be fixed-size: not resizable and not minimizable, as
its content neither reflows usefully nor benefits from being kept in the Dock.

Dismissal SHALL follow the host platform:

- On macOS the window SHALL follow the platform's About-box shape - icon,
  name, version and build, copyright and derivation, then credits, laid out
  in that order down the window, each line centred - and SHALL be dismissed by its close button or by
  `Escape`. It SHALL NOT carry an in-window Close or OK button, which a macOS
  About box does not have.
- On other platforms the window SHALL carry an explicit Close button in
  addition to `Escape`, since dismissal by window chrome alone is not the
  expectation there.

`Escape` SHALL close the About window on every platform.

#### Scenario: Escape closes it

- **WHEN** the About window has focus and the user presses `Escape`
- **THEN** the About window closes and no other window is affected

#### Scenario: Fixed size

- **WHEN** the user drags the About window's edge or corner
- **THEN** the window does not resize

#### Scenario: macOS dismissal

- **WHEN** the About window is shown on macOS
- **THEN** it shows the icon, name, version and build, copyright, derivation
  and credits in that order, every line centred, and offers no in-window
  Close or OK button

#### Scenario: Non-macOS dismissal

- **WHEN** the About window is shown on a platform other than macOS
- **THEN** it carries a Close button that closes the window

### Requirement: Every line of the window is localized

Every piece of user-facing text the About window shows - the app name, the
labels around version and build, the copyright line, and the credits - SHALL
come from the localization catalog rather than being written inline, so the
window can be translated without changing it.

Values that are not prose - the version number, the build identifier, and
third-party project names - are not translated, but the text around them
SHALL be.

#### Scenario: Text comes from the catalog

- **WHEN** the About window renders
- **THEN** each label, the copyright line, and the credits text resolve
  through the localization catalog
