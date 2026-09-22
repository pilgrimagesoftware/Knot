# Design

## Context

See proposal.md - Why. What shapes the approach is that almost nothing about
a menu item's shortcut is under this crate's control; it is decided by three
mechanisms in gpui and AppKit, and the work was mostly a matter of not
fighting them.

- **A shortcut is looked up by action, not set on the item.** Building the
  menu, gpui asks the keymap for the bindings of each item's action and puts
  the first one's keystroke on the `NSMenuItem`
  (`gpui-pre-macos-0.3.5/src/platform.rs:322-443`). There is no
  "`MenuItem::action(...).key("cmd-n")`". Two items sharing an action share
  its shortcut.
- **`NoAction` is not an inert placeholder in a keymap.** `Keymap::add_bindings`
  routes a binding whose action is `NoAction` into `disabled_binding_indices`
  (`gpui-pre-0.3.5/src/keymap.rs:66-70`): it is the "unbind this key"
  mechanism. So the ten standard items the port had parked on
  `gpui_kit::NoAction` could neither carry different shortcuts from each
  other nor be given one at all without suppressing that key elsewhere.
- **Enablement comes from validation, not from the item.** gpui never calls
  `setAutoenablesItems_`, so AppKit's default applies and every action item
  is validated on open: `validate_menu_item` answers from
  `App::is_action_available` (`platform.rs:1529`, `gpui-pre-0.3.5/src/app.rs:2451`),
  which walks the focused element's dispatch path for a listener of that
  action type. `.disabled(true)` on an action item is therefore advisory -
  it is what the item starts as, not what it stays. The Agents menu already
  relies on this (`crates/knot/src/agent_menu.rs:12-30`).
- **Binding precedence is depth-then-recency, and no context means deepest.**
  `Keymap::bindings_for_input` sorts by context depth and breaks ties in
  favour of the later-added binding, treating a context-less binding as
  deepest (`keymap.rs:150-160`). Our bindings are installed after
  `gpui_kit::init`, so a context-less binding of ours beats any toolkit
  binding on the same key.
- **gpui already owns the text keys.** `cmd-x`, `cmd-c`, `cmd-v`, `cmd-z`,
  `cmd-shift-z` and `cmd-f` are bound in its `Input` key context
  (`gpui-base-0.6.4/src/input/base/state.rs:244-296`) to actions the focused
  field handles.
- **The Agents menu's shortcuts have no platform source.** Its items are
  Knot's own. The Swift reference does give five of them keys, but not on
  its context menu - they sit in its File and Edit command groups
  (`Skwad/SkwadApp.swift:198`, `:277`, `:287`, `:295`).

## Goals / Non-Goals

**Goals:**

- Every menu item macOS gives a standard key equivalent shows it, including
  the items whose behavior is not implemented yet.
- No key a user already relies on changes hands.
- A rule the next person can apply without rediscovering the mechanisms
  above, rather than a table of ten decisions.

**Non-Goals:**

- Implementing the behavior behind New Workspace, Close Window, Enter Full
  Screen, Minimize, Zoom or Knot Help. Each wants a handler on the window
  that owns it.
- Adding menu items. Which items the bar carries is a separate question
  from which keys the ones it carries answer to.
- A `key_context` scheme for any of this. Nothing here needs to be scoped
  more finely than "the focused window answers it if it can".

## Decisions

### A named action per unimplemented standard item, marked `UNWIRED`

New Workspace, Close Window, Enter Full Screen, Minimize and Knot Help each
get their own action (`crates/knot/src/app_bootstrap.rs:127`) and their
conventional binding (`:301-305`). Nothing registers a handler, so
`is_action_available` answers false and AppKit draws each item disabled with
its shortcut greyed beside it - which is how macOS presents a standard item
an application does not currently offer, and what distinguishes "not yet"
from "never".

Alternatives considered:

- *Leave them on `NoAction` and add no shortcuts.* This is what the port did,
  and it is what the change exists to fix: ten bare items read as an
  application with no keyboard.
- *Bind the keys to `NoAction` directly.* Would have unbound those keys
  everywhere rather than displaying them - see Context.
- *Implement the behavior now so the items are live.* Each needs a window
  handler, which is a per-item, window-scoped change; bundling five of them
  into a keyboard change would make the keyboard rule hard to see and hard
  to revert. The actions are in place, so wiring one later is a handler
  registration and nothing else.

Zoom keeps `NoAction`. It has no reason to be named, because macOS gives it
no key equivalent.

### The Edit menu points at gpui's text actions, not at placeholders

`Undo`, `Redo`, `Cut`, `Copy` and `Paste` in the Edit menu are
`gpui_kit::base::input`'s own actions (`app_bootstrap.rs:240-245`), with no
bindings added by us.

This is forced, not preferred. A placeholder action of ours bound to `cmd-c`
would out-rank the `Input` binding by the precedence rule above, and copying
in every text field in the application would stop working. Pointing the menu
at the real actions is the only way to show those five shortcuts without
taking them.

It is also the better outcome: the items become live rather than decorative.
`is_action_available` enables them exactly when a text field is focused, so
`.disabled(true)` comes off - leaving it would grey five items that work.

The keystrokes still display, despite those bindings being scoped to a key
context the menu is not evaluated in, because the lookup is
`find_or_first`: with no binding matching the default context it falls back
to the first binding for the action (`platform.rs:344-360`).

**The terminal keeps `cmd-c`.** The terminal pane answers it with
copy-selection (`crates/knot/src/workspace_window/terminal_input.rs:50`),
and a menu key equivalent is offered to the menu before the window sees the
event. What saves it is that with no text field focused the Edit items
validate as unavailable, and AppKit does not let a disabled item consume a
key equivalent - `performKeyEquivalent:` declines and the event continues
down the responder chain. This is reasoned from documented AppKit behavior
and pinned by nothing automated; task 5.3 is the check that would observe
it, and it is the task worth doing first.

### Where the reference and the platform want the same key, the platform wins

Two of the Swift reference's five shortcuts collide with keys macOS has
already spoken for, and both go to the platform.

- **Close Agent `cmd-w`** is this port's Remove Agent
  (`crates/knot/src/app_state.rs:154-157` records the rename). `cmd-w` is
  Close Window. Remove Agent gets *no* key rather than a second-choice one -
  it is the destructive item, and an unfamiliar shortcut on an irreversible
  action is worse than no shortcut at all.
- **Fork Agent `cmd-f`** is Find everywhere else on the Mac, and gpui binds
  it to in-field Search. Fork Agent takes `cmd-alt-f`; `cmd-f` stays free.

The second case is the one that generalizes, so the spec states the rule
rather than the instance: no item of this menu holds a key the platform
reserves, *including one the port has no item for yet*. The port has no find
today - a menu item on `cmd-f` would not have shared the key with a find
added later, it would have taken it, because AppKit offers the keystroke to
the menu ahead of the window. Declining a reserved key costs one modifier;
reclaiming it later costs a fight with the menu bar.

### The Agents menu's bindings live beside its actions

`agent_menu_key_bindings` (`crates/knot/src/agent_menu.rs:86`) returns the
table, and `install_actions_and_keys` calls it. The alternative - listing
them with the application's own bindings in `app_bootstrap.rs` - puts the
keys a screen away from the actions and the menu they annotate, in a file
that has no other reason to know the Agents menu exists.

They carry no key context, so a workspace window answers them wherever focus
sits inside it. That is deliberate: the handlers are registered per selected
agent on the window's root element, so the keys are already inert exactly
when the menu items are greyed, and a context would add a second, redundant
gate.

### Tests assert the keymap, not the drawn menu

A menu item's key equivalent *is* the keystroke its action resolves to, so
pinning the resolution pins the menu
(`crates/knot/src/tests/menu_key_equivalents.rs`). The load-bearing test is
the Edit menu's: it asserts `cmd-c` still resolves to gpui's `Copy` after our
bindings are installed, which is what would fail loudly if someone added a
knot placeholder on a text key and broke every text field.

The assertions that name gpui's actions are macOS-only. gpui binds them to
`ctrl-c`, `ctrl-y` and friends off the Mac, so the keystrokes do not exist
there; the rule holds on every platform but the spelling does not, and the
menu bar these are key equivalents of is AppKit's. Our own bindings stay
asserted everywhere.

## Risks / Trade-offs

- **Ten shortcuts now displayed for items that do nothing.** → This is the
  macOS presentation for an unavailable standard item, and the items were
  already disabled; the change is that a user can now see what the key *will*
  be. The risk is that a greyed item reads as broken rather than pending -
  accepted, because the previous state read as absent, which is worse.
- **The Edit menu became functional inside a keyboard change.** → Named in
  the proposal rather than slipped in. It is not scope taken on by choice:
  showing those five shortcuts without hijacking them requires pointing at
  the working actions, and pointing at working actions makes them work.
- **The terminal's `cmd-c` fall-through is reasoned, not observed.** → Task
  5.3. If AppKit does consume the key equivalent on a disabled item, the
  terminal loses copy and the fix is a key context on the Edit items.
- **`cmd-alt-f` is Find & Replace in some Apple applications** (Pages,
  TextEdit, Xcode). → No conflict in this port or in gpui, but the same
  reasoning that moved Fork Agent off `cmd-f` would apply again if a
  find-and-replace ever surfaces. Noted rather than pre-solved.
- **A future multi-line input makes `cmd-f` matter.** → It is free, which is
  the whole point of the decision above.

## Open Questions

None. The one thing not settled by reading code is AppKit's behavior on a
disabled item's key equivalent, and that is a verification task with a named
fallback, not an open question about the design.
