# Design

## Context

- The menu bar is built in one place, `app_bootstrap::set_app_menus`, from an
  `AgentMenuSnapshot`. A gpui `Menu` is a static snapshot, so anything that
  changes what a menu lists requires rebuilding the whole bar.
- `AgentsMenuState` (a global) records which workspace window owns the bar and
  the snapshot it was built from. Each workspace window's `repaint_poll_tick`
  calls `refresh_agents_menu`, which rebuilds only when the owner or snapshot
  differs. Only the active window claims the bar; an inactive owner hands it
  back.
- `keymap::apply_and_refresh_menus` already rebuilds the bar after a rebinding,
  so key equivalents come from the live keymap.
- The navigation shortcuts' handlers are global (`keymap/handlers.rs`) and find
  their window through `cx.active_window()`. That is deliberate: a handler on
  the window root falls off the dispatch path once a panel replaces the focused
  composer. The consequence here is that `is_action_available` is always true
  for these actions, so AppKit would draw every View item enabled.
- The View menu is currently `Menu::new("View").items([])`, and macOS appends
  Enter Full Screen to it.

## Goals / Non-Goals

**Goals:**
- One description of the bar: the View menu is built in `set_app_menus`, next
  to the others.
- The View menu's enablement, checked state and submenus follow the owner
  window with no extra polling.

**Non-Goals:**
- Changing how the shortcuts dispatch, or moving their handlers.
- A general menu framework. This adds one menu to the existing snapshot model.

## Decisions

### Reuse the existing actions and global handlers

Each item uses the shortcut's own action (`ToggleDashboard`, `SelectAgent3`,
`SelectWorkspace1`, ...). gpui looks up a menu item's key equivalent by action,
so reusing them is what makes the item show the current binding, and it
guarantees that the item and the key do the same thing.

Alternative: menu-only actions that forward to the shortcut. Rejected. They
would carry no binding, so the items would show no key, and binding them too
would double every chord.

### Enablement through `disabled`, computed from the snapshot

The handlers are global, so availability cannot disable anything. The View
menu's items therefore set `MenuItem::disabled` from the snapshot, the way File
> New Workspace does today. A disabled item still shows its key equivalent.

Alternative: register the handlers on the window root as well, and let
availability decide. Rejected. A global listener makes the action available
everywhere, so the root handler would change nothing, and removing the global
handler reintroduces the panel dispatch bug described in `handlers.rs`.

### Extend the snapshot, not a second global

`AgentMenuSnapshot` becomes the menu bar's snapshot. It gains a `view` part,
computed in the same pass as the Agents menu's facts:

- `workspace_window: bool` - whether a workspace window owns the bar;
- `view_mode` - agent view, Dashboard or Pull Requests, for the checkmarks and
  Jump to Bottom;
- `selected_agent_mode` - none, panel or terminal;
- `agents: Vec<(Uuid, String)>` - the first nine sidebar agents, and which one
  is selected;
- `workspaces: Vec<(Uuid, String)>` - the first nine workspaces, and which one
  the owner shows.

The existing compare-then-rebuild in `refresh_agents_menu` then covers every
View change: a selection, a panel toggle, an agent added or renamed. Renaming
the type and state to `MenuBarSnapshot` / `MenuBarState` is part of the change,
since they no longer describe only the Agents menu.

Alternative: a separate `ViewMenuState` global. Rejected. The bar is rebuilt as
a whole, so two globals would each rebuild it and could race to overwrite each
other's half.

### Workspace list without a workspace window

Select Workspace has to be correct with no workspace window focused. When no
workspace window owns the bar, `workspaces` is still filled from the store.

The workspace manager, where workspaces are created, renamed, deleted and
reordered, has no poll tick, and adding one for this would be the wrong fix.
Each of its mutations already persists and notifies; after that it calls an
app-level `refresh_menu_bar_workspaces(cx)`. That function recomputes only the
workspace list, keeps the current owner and the rest of the snapshot, and
rebuilds only if the list changed. A workspace window's own tick recomputes the
list as part of its snapshot, so a change made through MCP or an import also
reaches the submenu once any workspace window polls. With every window closed
nothing can change the list, so nothing needs to poll.

Alternative: a poll tick on the manager. Rejected. It would re-read the store
every frame to catch changes the manager itself makes, and the manager already
knows when it makes one.

### Menu construction lives beside the Agents menu

A `view_menu(&snapshot) -> Menu` function in a new sibling file
(`crates/knot/src/view_menu.rs`) builds the items, as `agents_menu` does in
`agent_menu.rs`. `app_bootstrap.rs` keeps only the call, which keeps it under
the 700-line limit. Labels go through `knot_core::l10n::t` under `menu.view.*`.

### New Agent is a fixed ⌘T shortcut on a global handler

`NewAgent` joins `keymap/fixed.rs`, so the validator already rejects a
customization onto ⌘T. Its handler is global and reaches the window through
`in_active_workspace`, for the same reason the navigation handlers are: a
root-element handler is off the dispatch path while a panel is showing. File >
New Agent… takes `disabled` from the snapshot's workspace-window flag.

Alternative: make it configurable. Rejected. Nothing asked for it, and the
reference's key is fixed.

### ⌘-hold hints: window state, a timer, and a cached label set

The workspace window tracks `command_held_since: Option<Instant>` and
`show_key_hints: bool`:

- The root element's `on_modifiers_changed` sets `command_held_since` when ⌘
  goes down and clears both when it goes up.
- A key-down listener in the capture phase clears both, so that ⌘C cancels the
  hold. It does not stop propagation.
- `show_key_hints` turns on from a 500 ms timer spawned when ⌘ goes down. The
  timer checks that the hold it was started for is still the current one and
  then notifies the view. A per-frame elapsed-time check would need the window
  to keep repainting while nothing else changes.
- Window deactivation (`observe_window_activation`) clears both. macOS does not
  deliver the ⌘ key-up to a window that lost key status during ⌘Tab, so without
  this the hints would still be showing when the user returns.

The labels come from `Resolved`, stored on the window as a `SidebarKeyHints`
(Dashboard, Pull Requests, New Agent and nine agent chords as strings). It is
rebuilt when the keymap is applied, the same moment the menu bar is rebuilt, so
the render path formats nothing and reads no settings. This follows the
no-work-on-the-render-path rule in `.claude/rules/rust-structure.md`.

Rendering: at full width a hint is an absolutely positioned, right-aligned
label inside the row, over its trailing content. In compact layout it is an
absolutely positioned badge at the avatar's or icon's corner. Being absolutely
positioned is what keeps row sizes unchanged.

Alternative: show hints for as long as ⌘ is held, with no delay. Rejected. The
hints would flash on every ⌘C, ⌘V and ⌘Tab.

Alternative: show hints only while exactly the family's modifier is held (⌥⌘
for agents). Rejected. Users would have to know the chord before the hint could
show it to them.

## Risks / Trade-offs

- [Rebuilding the bar more often: every selection or panel toggle now changes
  the snapshot] → Rebuilds already happen on selection changes, and a panel
  toggle is a user action at human speed. The compare still skips every
  unchanged tick.
- [macOS could stop appending Enter Full Screen once the View menu has items]
  → It adds the item to a menu titled View whatever else it holds. A test
  asserts the menu declares no full-screen item of its own, and the manual
  check confirms exactly one appears.
- [Agent names in the snapshot are read on every poll] → The Agents menu's
  facts are already read under the same store lock. The list is capped at
  nine.
- [gpui may not deliver `ModifiersChangedEvent` to a window whose focused
  element is the terminal pane] → The listener is on the root element, which
  is an ancestor of every pane. Task 5.1 verifies this with the terminal
  focused before any rendering work starts.
- [⌘T in the terminal pane: a shell user may expect it to reach the program]
  → The terminal forwards no ⌘ chords today (they are app shortcuts on macOS),
  so ⌘T takes nothing that currently works.
- [A disabled item whose global handler still exists: the key keeps working
  where the item is disabled] → Intended. The keys already do nothing outside a
  workspace window, and the item's disabled state reflects that.
