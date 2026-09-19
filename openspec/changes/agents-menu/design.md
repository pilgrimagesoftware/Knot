## Context

See proposal.md — Why. The mechanisms this rests on, all confirmed in gpui
rather than assumed:

- `Window::is_action_available` walks the **last rendered frame's** dispatch
  tree along the focus path, and gpui's menu validation callback calls it for
  every menu item. So an `.on_action` handler attached conditionally during
  render decides whether macOS draws that item enabled - no menu rebuild
  needed, and it updates on the next frame.
- `App::is_action_available` ORs the window's answer with "is there a global
  listener for this action". An action registered globally is therefore
  *always* enabled, which is why these actions must be window-level only.
- A `Menu` handed to `cx.set_menus` is a static snapshot. Submenu contents
  do not re-evaluate, so anything dynamic must be rebuilt by calling
  `set_menus` again.
- `agent_context_menu_entries(facts) -> Vec<AgentMenuEntry>` and
  `run_agent_menu_action(entry, targets, window, app)` already exist, pure
  and shared, from the context-menu parity work.

## Goals / Non-Goals

**Goals:**

- One item set behind both menus.
- Enablement that follows the selection without the window having to tell the
  menu bar anything.

**Non-Goals:**

- Keyboard shortcuts. Worth having, but each one is a decision about which
  key and what it collides with, and the menu is useful without them.
- Menus for anything else in the bar. The File/Edit/View/Window placeholders
  stay as they are; this change writes down only the menu it adds.
- Acting on multiple agents. The sidebar has one selection.

## Decisions

### Disable rather than omit, unlike the context menu

The context menu omits what does not apply, and should: it opens at the
pointer, is read top to bottom each time, and an omitted item costs nothing
because nothing was in that position a moment ago.

A menu bar is the opposite. It is navigated by position from memory, and an
item set that changes shape per selection makes that impossible - the user
reaches for the fourth item and gets a different action. So the Agents menu
always shows every item and disables what does not apply.

This is a deliberate divergence, written into the spec rather than left for a
reader to spot as an inconsistency.

### Enablement through action availability, not by rebuilding the menu

The workspace window registers its Agents-menu handlers while an agent is
selected, on an element in the focus path. macOS then asks gpui per item, and
gpui answers from the rendered frame's dispatch tree. Selecting an agent
re-renders, the handlers appear, the items enable.

The alternative - calling `set_menus` on every selection change - rebuilds the
whole bar to change one menu's greying, and races the frame it depends on.
Reserved for the case that genuinely needs it (below).

*This is also why the actions must not be registered globally.* A global
listener makes `is_action_available` true unconditionally, and every item
would be permanently enabled - including with no window open at all.

### One action type per item, not one action carrying a discriminant

Twelve unit actions (`AgentMenuEdit`, `AgentMenuFork`, …) rather than one
action with an entry field. They are what gpui's menu and keybinding systems
address, they show up by name in a keymap when shortcuts arrive, and each maps
to exactly one `AgentMenuEntry`, so the handler is a match with no fallthrough.

### Dynamic submenus force a rebuild, and only they do

Move to Workspace and Markdown Files depend on data a static `Menu` cannot
re-read. The menu bar is therefore rebuilt when the workspace list or the
selected agent changes - a `set_menus` call from the workspace window's
existing repaint poll when the facts behind those two submenus differ from
the ones last built.

Everything else - which items exist, whether they are enabled - is handled by
the availability mechanism above and needs no rebuild.

### Dialogs must be deferred, for a second reason

The destructive items open confirmations, and the context menu already defers
them because a `PopupMenu` dismisses itself and takes an inline dialog with
it. From the menu bar the same `cx.defer` is needed for a different reason:
`App::dispatch_action` runs the handler *inside* the active window's update,
so opening a dialog on that window is re-entrant and gpui refuses it with
"window not found" - the About Knot bug, exactly.

Reusing `run_agent_menu_action`, which already defers, means neither surface
can get this wrong independently.

## Risks / Trade-offs

- **A disabled item gives no reason.** "Fork Agent" greyed out does not say
  "because this is a companion" → accepted; it matches every other macOS menu,
  and the context menu remains the surface that shows only what applies.
- **The rebuild could flicker the menu bar.** Calling `set_menus` replaces the
  whole bar → it is called only when the submenu facts actually change, not
  per poll, so it fires on workspace creation and markdown display rather
  than continuously.
- **Two surfaces for the same actions double the manual test surface** → the
  shared entry list and shared handler mean the divergence risk is in the
  rendering only, which is what the first task covers.

## Open Questions

None.
