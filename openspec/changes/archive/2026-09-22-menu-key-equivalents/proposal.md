# Proposal

## Why

Knot's menu bar shows a shortcut beside four items - Settings, Hide Knot,
Hide Others and Quit Knot - and beside nothing else. Every other standard
item macOS gives a key equivalent (New, Close Window, Undo, Redo, Cut, Copy,
Paste, Enter Full Screen, Minimize, and the Help item) is drawn bare, which
reads as an application with almost no keyboard at all. Users learn these
keys from the system, not from the app, so an empty right column is read as
"not available here" rather than "not implemented yet".

Two of those menus deserve separating. The Edit menu's five items are not
merely missing their shortcuts - the actions behind them exist and work. gpui
binds Cut, Copy, Paste, Undo and Redo for a focused text field already, and
the Edit menu is pointing at `NoAction` placeholders instead, so the one menu
whose items could work today is the one that does nothing.

## What Changes

- Every menu item macOS gives a standard key equivalent SHALL show that key
  equivalent: New Workspace ⌘N, Close Window ⌘W, Undo ⌘Z, Redo ⇧⌘Z, Cut ⌘X,
  Copy ⌘C, Paste ⌘V, Enter Full Screen ⌃⌘F, Minimize ⌘M, Knot Help ⌘?.
- About Knot, Show All and Zoom SHALL keep no shortcut. macOS gives those
  three none either, so adding one would be an invention rather than a
  convention.
- The Edit menu's five items SHALL dispatch gpui's own text actions rather
  than placeholders, which both gives them their shortcuts and makes them
  work: they enable with a text field focused and grey out elsewhere.
- The items that have no behavior yet SHALL stay disabled. Their shortcut is
  drawn greyed beside the label, which is how macOS presents a standard item
  an application does not currently offer.
- The Agents menu SHALL take the shortcuts the Swift reference gives its
  items: New Shell Companion ⇧⌘S, Fork Agent ⌥⌘F, Duplicate Agent ⌘D and
  Restart Agent ⌘R. No platform convention names a key for these - they are
  Knot's own items - so the reference is the source, except where it wants a
  key the platform has already spoken for.
- Remove Agent SHALL keep no shortcut. The reference calls it Close Agent and
  gives it ⌘W, which Close Window has here; the destructive half of a very
  common keystroke is not the place to depart from the platform.
- Fork Agent SHALL take ⌥⌘F rather than the reference's ⌘F, and ⌘F SHALL stay
  free. ⌘F is Find on every other Mac application - gpui binds it to in-field
  Search already - and a menu item's key equivalent is claimed by AppKit
  ahead of the window, so taking it would spend the platform's find key on a
  Knot action before this port has a find of its own to put there.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `app-menu`: today it specifies the Agents menu's items and their
  enablement, and says nothing about keys. This adds the rule that the bar's
  standard items carry the platform's key equivalents, and records the four
  the Agents menu takes from the Swift reference - which is what stops the
  next person adding an item without one.

## Impact

- `crates/knot`: `app_bootstrap.rs` - one `actions!` block for the
  not-yet-wired standard items, five bindings, and the menu definitions in
  `set_app_menus` pointing at real actions instead of `NoAction`;
  `agent_menu.rs` - a table of the four bindings its own actions take from
  the reference, beside the actions they name.
- No change below the UI. Nothing gains behavior except the Edit menu, whose
  actions already existed and were already reachable by keyboard.

## Non-Goals

- Implementing New Workspace, Close Window, Enter Full Screen, Minimize,
  Zoom or Knot Help. Each needs a handler on the window that owns the
  behavior, which is a window-scoped change per item; this change is the
  keyboard contract, not the behavior behind it.
- Adding Select All to the Edit menu, or any other item the bar does not
  already have. Which items the bar carries is a separate question from
  which keys the ones it carries answer to.
- The reference's shortcuts that do not land on a menu the port shows one
  on: New Agent ⌘T and Broadcast ⇧⌘B belong to the sidebar's background
  menu, which has no menu-bar presence; Open in <app> ⇧⌘O was one item for
  the default application where this port has a submenu of all of them. Each
  needs a menu item before it can need a key.
