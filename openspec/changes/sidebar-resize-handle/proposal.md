# Proposal

## Why

The workspace window's agent sidebar is a hard-coded 250px column. A user with
long agent names, deep folder paths or a narrow display cannot widen it, and a
user who wants the terminal as wide as possible cannot narrow it. The Swift
reference has been draggable since the beginning (`ContentView.swift` -
`sidebarWidth`, a 6px handle clamped to 80-400px), so this is a porting gap
rather than a new feature.

## What Changes

- Add a draggable divider between the agent sidebar and the content column.
  Dragging it sets the sidebar's width; the content column takes the rest. The
  divider shows the platform's column-resize cursor on hover and highlights
  while it is being dragged.
- Clamp the sidebar between 120px and 400px. The lower bound is above the Swift
  reference's 80px because the Rust window puts the traffic lights inside the
  sidebar column, which reserves 80px on its own.
- Persist the width as a new `sidebar_width` scalar in `Settings`, defaulting to
  250. It is written when a drag ends, not during it, and read when a workspace
  window opens. A persisted value outside the clamp is brought into range on
  load rather than rejected.
- Render the sidebar's rows in a compact layout below 160px, porting
  `SidebarView.swift`'s `isCompact`: an agent row becomes its avatar alone,
  centered, with the state dot overlaid on it and the agent's name as its
  tooltip; the dashboard row becomes its icon alone; the sidebar's title bar
  drops the app-name label and keeps the icon; the "New agent" button becomes
  icon-only, keeping its tooltip. Selection, the dimming of a stopped agent, the
  companion indent and every context menu behave as they do at full width.
- Add no menu item, keyboard shortcut or settings control for the width. The
  divider is the only way to set it, as in the reference.

### Non-goals

- Collapsing the sidebar entirely. The Swift reference has a `sidebarVisible`
  toggle and a collapse button; the Rust port has neither, and adding a hide or
  a toggle is its own change.
- A resizable divider anywhere else - between split terminal panes, or around
  the panel's input area.
- Per-workspace widths. One width applies to every workspace window, unlike
  window bounds, which are already stored per workspace.
- Any change to what a row shows at full width.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-list-ui`: new requirements for the sidebar's width - the divider, its
  clamp, and what the sidebar shows when narrower than the compact breakpoint.
  The capability already owns the workspace sidebar; this is the sidebar's size
  and the row layout that follows from it.
- `settings-persistence`: a new `sidebar_width` scalar with its default and its
  load-time clamping, listed alongside the other scalars the store holds.

## Impact

- `knot-core`: `consts.rs` (default width, minimum, maximum, compact
  breakpoint), `settings/store.rs` (the `sidebar_width` field, decode tolerance
  and the clamp on load, tests).
- `knot`: `workspace_window/render/mod.rs` (the two columns become a resizable
  group), `workspace_window/render/sidebar.rs` plus a new sibling module for the
  compact row, `workspace_window/render/overview.rs` (compact dashboard row),
  `workspace_window/mod.rs` and `open.rs` (the window holds the resize state
  entity and subscribes to the end of a drag).
- No new dependency: `h_resizable`, `resizable_panel` and `ResizableState` are
  already re-exported by `gpui-kit` 0.6, and its resize handle is absolutely
  positioned, so adding one does not shift either column's contents.
- A window that writes the width must re-read `Settings` from disk first, as
  the agent editor and the bench already do - every window holds its own
  snapshot, and persisting a stale one would discard another window's edits.
