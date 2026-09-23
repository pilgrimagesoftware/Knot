# Tasks

## 1. Refresh preferences in place

- [x] 1.1 Add a `Settings` method that re-reads the preferences document from
      the value's own store paths and adopts its scalars, preserving every
      `#[serde(skip)]` collection and the paths themselves; verify with a test
      that sets a scalar on disk, populates the collections in memory, reloads,
      and asserts the scalar moved and the collections did not.
- [x] 1.2 Verify the reload is tolerant of a missing or malformed preferences
      document on the same terms as load, so a refresh cannot be the thing that
      fails a running app.

## 2. Deliver the change to open windows

- [x] 2.1 Let the window registry enumerate the live workspace views, dropping
      entries whose window has closed.
- [x] 2.2 Give `WorkspaceWindow` a method that adopts fresh preferences and
      requests a repaint.
- [x] 2.3 Route the settings window's persist path through a broadcast that
      refreshes every open workspace window; verify no I/O lands on the render
      path.

## 3. Verification

- [x] 3.1 Add a test that drives the real path: a workspace window open with
      compact mode off, a preferences write turning it on, and the window's
      own `PanelStyle` reading the new value.
- [x] 3.2 Add a test that a refresh leaves the window's roster snapshot alone.
- [x] 3.3 Run `make` and confirm formatting, size check, lint and the full test
      suite pass.
