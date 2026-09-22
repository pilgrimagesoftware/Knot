# Proposal

## Why

Knot's windows can be opened but not found again. Opening the same workspace
twice from the manager, the Command Center or a double-click produces a second
window for the same workspace rather than raising the first; a workspace window
restored onto a display that is no longer attached is drawn where the user
cannot reach it; and the Command Center - the one view that shows every
workspace at once - has neither a menu item nor a shortcut to open it, and
clips its own content when the grid outgrows the window.

The menu bar has the matching problems. The View menu shows "Enter Full Screen"
twice, because Knot declares a dead item of its own beside the live one AppKit
inserts. The Window menu offers only Minimize and Zoom - both disabled - and
then AppKit's auto-populated list of open windows, so the two destinations a
user actually wants (the Command Center, the workspace manager) are absent from
the menu and unreachable by keyboard.

## What Changes

- One workspace window per workspace: a second open request for a workspace
  that already has a window activates that window instead of creating another,
  from every route - the manager list, a double-click, a Command Center card,
  and the agent editor's post-create jump.
- One Command Center window, reusing the same singleton-and-activate pattern
  the About, Settings and Import windows already use.
- Restored workspace window bounds are constrained to the currently attached
  displays before the window opens, so a window saved on a disconnected display
  reopens somewhere the user can grab it. A window is never placed with less
  than a usable corner on screen.
- The Command Center's agent grid scrolls when it overflows the window.
- The View menu shows exactly one Enter Full Screen item, AppKit's, which is
  enabled and works. Knot's own item goes, and with it the `EnterFullScreen`
  action and its ⌃⌘F binding - the key stays live, held by the platform's item
  rather than by Knot's keymap. ⌃⌘F is a key macOS reserves, and `app-menu`
  already forbids a Knot item from holding one.
- The Window menu gains a Command Center item (⌥⌘0) and a Workspaces item (⌘0),
  both wired, above Minimize and Zoom and separated from AppKit's auto-appended
  list of open windows.

## Capabilities

### New Capabilities
- `window-lifecycle`: how many windows a given target may have, how a repeat
  open request resolves to activating the existing window, and how a restored
  window's saved bounds are reconciled with the displays actually attached.

### Modified Capabilities
- `app-menu`: the View menu no longer declares its own Enter Full Screen item
  and ⌃⌘F is no longer Knot's to bind; the Window menu gains Command Center and
  Workspaces items with key equivalents, and a stated order that keeps them
  above the window list AppKit appends.
- `dashboard`: the Command Center is a single window that activates rather than
  reopening, is reachable from the Window menu, and scrolls its grid.

## Impact

- `crates/knot/src/app_bootstrap.rs` - menu declarations, action set, key
  bindings. The View menu is declared with no items of its own;
  `EnterFullScreen` leaves the `actions!` list and `cx.bind_keys`;
  `OpenCommandCenter` and `OpenWorkspaces` are added and wired.
- `crates/knot/src/workspace_window/open.rs` - the open path gains the
  registry check and the bounds reconciliation.
- `crates/knot/src/window_options.rs` - `workspace_window_options` clamps
  saved bounds against the attached displays instead of applying them verbatim.
- `crates/knot/src/command_center.rs` - singleton handle, scroll container.
- `crates/knot/src/workspace_manager/` and `crates/knot/src/command_center.rs`
  call sites route through the new open path rather than calling
  `WorkspaceWindow::open` directly.
- `crates/knot-core/locales/en.yml` - keys for the two new menu items.
- `crates/knot/src/tests/menu_key_equivalents.rs` - the ⌃⌘F assertion goes,
  because the file records what Knot claims on the keyboard and ⌃⌘F is no
  longer one of those claims; ⌥⌘0 and ⌘0 assertions arrive.
- No new dependencies. No change to the persisted settings schema:
  `SavedWindowBounds` is still written as-is and only interpreted differently
  on read.

## Non-Goals

- Restoring which workspace windows were open at launch. This change governs
  windows opened during a session, not session restoration.
- Making Minimize, Zoom, New Workspace, Close Window or Knot Help work. They
  stay disabled placeholders under `app-menu`'s existing requirement. Enter
  Full Screen leaves that group by being handed to the platform, not by being
  implemented; the rest stay.
- Localizing the rest of the app menu bar's hardcoded English titles. That
  inconsistency is real but separate.
