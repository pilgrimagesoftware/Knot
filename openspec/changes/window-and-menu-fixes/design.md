# Design

## Context

See `proposal.md` - Why. The relevant current state:

- `WorkspaceWindow::open` / `open_with_selection` (`crates/knot/src/workspace_window/open.rs:30-198`)
  calls `cx.open_window(...)` unconditionally. Nothing anywhere maps a
  workspace id to a window handle. The new window stores its own handle on
  itself (`open.rs:115`) for local checks only.
- Three windows already implement singleton-and-activate, all with the same
  idiom: About (`about_window/window.rs:46-61`), Settings
  (`settings_window/mod.rs:52-60`), Import (`import_window/window.rs:203-224`).
  Each holds an `Rc<RefCell<Option<AnyWindowHandle>>>` captured by the closure
  registered at action-install time, and probes liveness with
  `existing.update(cx, |_, window, _| window.activate_window()).is_ok()` - a
  closed window's `update` fails, so no close observer is needed.
- `workspace_window_options` (`window_options.rs:57-74`) applies
  `SavedWindowBounds` verbatim. GPUI's macOS backend resolves the target
  `NSScreen` by stored display id, falls back to the primary screen when that
  display is gone, and then adds the saved origin on top of the primary
  screen's origin with no clamping. Only workspace windows persist bounds;
  every other window centers on each open.
- `set_app_menus` (`app_bootstrap.rs:207-262`) declares the whole menu bar and
  is re-run whenever the Agents-menu snapshot changes
  (`workspace_window/menus/menu_bar.rs:109`). The View menu declares one
  `EnterFullScreen` item, permanently `.disabled(true)` because no handler is
  registered; ⌃⌘F is bound to it (`app_bootstrap.rs:310`). AppKit inserts its
  own live Enter Full Screen item because it identifies one by the
  `toggleFullScreen:` selector, which Knot's custom action is not.
- GPUI makes a top-level `Menu::new("Window")` the AppKit `windowsMenu` purely
  by name match, and AppKit then appends and maintains the open-window list
  below whatever items were declared.
- `CommandCenterWindow::open` (`command_center.rs:44-67`) has one call site, a
  toolbar button in the workspace manager (`workspace_manager/render.rs:293-304`).
  No action, no menu item, no binding.

Constraints: no `.rs` file over 700 lines; `mod.rs` declares only; user-facing
text through `knot_core::l10n::t`; no I/O on the render path; no crate-wide
`allow`.

## Goals / Non-Goals

**Goals:**

- One place that answers "is a window for X already open, and if so, raise it",
  reused by workspace windows, the Command Center and the workspace manager,
  rather than a fourth hand-rolled copy of the About/Settings/Import idiom.
- Bounds reconciliation as a pure function over `(saved, displays)`, unit
  testable without a window.
- Menu changes confined to `set_app_menus` and the actions/bindings block, so
  `menu_key_equivalents` keeps being the contract for what Knot claims on the
  keyboard.

**Non-Goals:**

- Retrofitting About/Settings/Import onto the new registry. They work; folding
  them in is a follow-up, not a prerequisite.
- A general window manager. The registry keys what exists today and nothing
  more.
- Persisting Command Center or manager bounds.

## Decisions

### A single `WindowRegistry` in `cx.global`, keyed by a `WindowKey` enum

A `WindowRegistry` global holds `HashMap<WindowKey, AnyWindowHandle>` where
`WindowKey` is `Workspace(Uuid) | CommandCenter | WorkspaceManager`. One method,
`activate_or_open(key, cx, open_fn)`, does the probe-then-open dance the three
existing singletons do by hand, and a second, `activate(key, cx) -> bool`, lets
a caller that needs to also select an agent do the extra work only on the hit
path.

- *Why a global*: the call sites are spread across `workspace_manager`,
  `command_center` and `app_bootstrap`, none of which owns the others. Threading
  an `Rc<RefCell<..>>` through all of them would mean adding a field to three
  view structs and a parameter to four constructors; a `cx.global` is the
  pattern GPUI offers for exactly this and keeps the change local to the open
  path.
- *Alternative - store the handle on `AgentStore`*: rejected. The store is
  shared with `knot-agents`, which is UI-toolkit-free; `AnyWindowHandle` is a
  GPUI type and does not belong in it.
- *Alternative - keep the per-window `Rc<RefCell<..>>` idiom and add a fourth
  and fifth*: rejected on the same "three times is a rule" grounds that produced
  `diff_stats.rs`. Workspaces need a map, not a slot, so the idiom does not
  extend anyway.
- *Closed windows*: no close observer. A stale entry is detected the way the
  existing code detects it - `handle.update(..)` returns `Err` - and is removed
  at that point, so the map self-cleans on the next request for that key. This
  is a correctness point, not just tidiness: `Workspace(Uuid)` entries would
  otherwise accumulate across a long session.

### Agent selection on activate is a method on the existing view

`activate_or_open` returns enough for the caller to distinguish "raised an
existing window" from "opened a new one". On the raise path the Command Center's
card handler downcasts the handle to `WindowHandle<WorkspaceWindow>` and calls
the same selection method `open_with_selection` uses on the fresh path, so one
implementation of "show this agent" serves both. This keeps the spec's "leaving
the window showing what a freshly opened one would have shown" true by
construction rather than by two code paths agreeing.

### Bounds reconciliation is a pure function in `window_options.rs`

`reconcile_bounds(saved: SavedWindowBounds, displays: &[Bounds<Pixels>]) -> Option<Bounds<Pixels>>`,
returning `None` for "use the default placement". The rules are the spec's, in
order: contained → unchanged; size exceeds every display → `None`; overlaps one
or more → translate onto the display of greatest overlap by the smallest offset
that puts the title-bar strip inside it; overlaps none → `None`.

- *Why a pure function*: it is the only part with real logic and the only part
  that can be tested without opening a window. Displays come from
  `cx.displays()` at the call site; the function takes rectangles.
- *"A graspable part of the top edge"*: implemented as a constant strip height
  in `consts.rs` (title bar plus a margin) that must lie inside the display's
  visible frame, plus a minimum horizontal overlap so the window cannot be
  pushed to a sliver at the edge. Naming it a constant rather than inlining it
  keeps the two numbers in the one place the project keeps constants.
- *Why translate rather than resize*: stated in the spec - a size the user chose
  is information. Resizing also interacts badly with the write-back observer,
  which would then persist the shrunken size.
- *Write-back*: unaffected. `cx.observe_window_bounds` (`open.rs:166-192`) fires
  on user moves and resizes; opening at reconciled bounds does not itself
  trigger it. If it turns out to fire on the initial placement, the observer
  gains a "skip the first notification" guard rather than the reconciliation
  writing back - the spec requires the remembered bounds to survive a
  disconnected display.
- *Alternative - clamp inside GPUI*: not ours to change, and it would apply to
  every GPUI app rather than to Knot's one window type that restores bounds.

### Hand Enter Full Screen to AppKit entirely

Remove the item from the View menu, the name from `actions!`, and the ⌃⌘F
binding. The View menu is then declared with no items of its own; AppKit creates
and populates it.

- *Why*: `app-menu` already forbids a Knot menu item from holding a key the
  platform reserves, on the grounds that AppKit claims a menu key equivalent
  ahead of the window and so takes it rather than shares it. That rule moved
  Fork Agent off ⌘F. ⌃⌘F is such a key, and the rule applies unchanged.
- *The shortcut is not lost*: AppKit's item carries ⌃⌘F itself, so the key
  keeps working. What goes is Knot's declaration of it, not its function.
- *Alternative - keep the item, wire it to `Window::toggle_fullscreen()`
  (`gpui-pre-0.3.5/src/window.rs:6318`), and suppress AppKit's with the
  `NSFullScreenMenuItemEverywhere` user default*: workable, and rejected. That
  default is documented in AppKit's 10.11 release notes and nowhere current; if
  a future macOS ignores it, AppKit re-adds its item, that item claims ⌃⌘F
  ahead of Knot's keymap, and Knot's binding goes dead silently with its test
  still passing. Leaving the key with the platform is the arrangement that
  cannot break that way.
- *What the platform supplies for free*: the label flipping between Enter and
  Exit Full Screen, which would otherwise need a full-screen-change observation
  GPUI may not expose; translation into the system language, which Knot cannot
  currently match because the menu bar's titles are hardcoded English and only
  `en.yml` ships; and correct targeting through the first responder, so the
  settings, about and import windows work without Knot resolving "the focused
  window" itself.
- *Alternative - wire `EnterFullScreen` and keep the item without suppressing
  AppKit's*: rejected outright. AppKit matches on the `toggleFullScreen:`
  selector, which a GPUI action is not, so it would still append its own. Two
  working items is not better than one working and one dead.
- The ⌃⌘F assertion at `crates/knot/src/tests/menu_key_equivalents.rs:92` is
  deleted. That file records what Knot claims on the keyboard; ⌃⌘F is no longer
  one of those claims, so there is nothing there to assert.

### The two new Window-menu items are ordinary actions

`OpenCommandCenter` and `OpenWorkspaces` join the `actions!` list, get
`cx.on_action` handlers registered in `install_actions_and_keys` alongside the
Settings/About/Import ones, and get `cmd-alt-0` / `cmd-0` bindings. Their
handlers call `WindowRegistry::activate_or_open`, which is also what the
manager's toolbar button and every workspace-open call site now call - so
"opened from the menu" and "opened from the button" are the same code.

- *Why ⌥⌘0 and ⌘0*: the user asked for them, and macOS reserves neither. ⌘0 is
  conventionally "reset zoom" in browsers and editors; Knot has no zoom level,
  so the key is free here.
- The manager's toolbar button stays. Removing it would take the Command Center
  out of the one window a first-time user is already looking at.

### Command Center scrolling

The grid moves into a scroll container in `command_center.rs`'s render, sized to
the window's content area with the sections as its content. The in-workspace
dashboard view is a separate render path in `workspace_window` and is out of
scope here; if it has the same defect it is its own change.

## Risks / Trade-offs

- **A `cx.global` is process-wide state, and a test that opens windows leaks
  entries into the next test** → keep the registry's API total (every operation
  valid on an empty map) and have the reconciliation logic - the part worth
  testing - live outside it as a pure function, so tests do not need to open
  windows to cover the interesting behavior.
- **`handle.update()` as a liveness probe also activates the window as a side
  effect** → that is what the existing three singletons rely on, so it is
  established behavior here, but it means "is it open?" cannot be asked without
  raising it. The registry therefore exposes no separate `is_open`; callers that
  only want to know are not a case that exists today.
- **`cx.displays()` reports the configuration at open time; a display can be
  disconnected while a window is open** → out of scope. The spec governs
  placement at open, and macOS already relocates windows on display removal.
- **Knot has no full-screen command of its own afterwards, so a Knot-specific
  full-screen behavior would need the item back** → adding it back is a small,
  well-understood change, and it would need AppKit's `toggleFullScreen:`
  selector to avoid the duplicate anyway, which is a different design from the
  one removed here.
- **Reusing a window instead of opening one changes what a double-click does for
  a user who wanted two views of the same workspace** → the spec makes this the
  intended behavior, and a workspace's split-pane layout is the supported way to
  see two things at once.
- **`set_app_menus` is re-run on every Agents-menu snapshot change** → the new
  items are declared in the same function, so they are rebuilt identically each
  time; there is no state in the menu to lose. Worth a check that re-running it
  does not disturb AppKit's appended window list.
