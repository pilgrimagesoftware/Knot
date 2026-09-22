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

The Agents menu is untouched. Its items are Knot's own, so no platform
convention names a key for them; the Swift reference does assign some
(`Skwad/SkwadApp.swift:188-355`), and adopting that set is a separate
decision from following the platform's.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `app-menu`: today it specifies the Agents menu only. This adds the rule
  that the bar's standard items carry the platform's key equivalents, which
  is what stops the next person adding an item without one.

## Impact

- `crates/knot`: `app_bootstrap.rs` only - one `actions!` block for the
  not-yet-wired standard items, five bindings, and the menu definitions in
  `set_app_menus` pointing at real actions instead of `NoAction`.
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
- Key equivalents for the Agents menu - see above.
