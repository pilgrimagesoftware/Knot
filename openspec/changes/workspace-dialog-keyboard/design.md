# Design

## Context

See proposal.md - Why. What shapes the approach:

- The dialog is not a window or a platform sheet. It is an overlay rendered
  inside the Workspaces window's own tree
  (`crates/knot/src/workspace_manager/mod.rs:334-395`): an absolutely
  positioned `div` over the list, holding a title, a `gpui_kit` `Input`, and
  Cancel / Create buttons.
- Its two actions already exist as methods and are already reachable by
  click: `confirm_workspace_dialog` (`:114`) and `cancel_workspace_dialog`
  (`:104`). Neither needs to change.
- Validation already exists, but only inside `save_name` (`:41-47`), which
  sets `self.error` when the trimmed name is empty. The button has no
  disabled state; the error string is rendered as a sibling of the overlay
  rather than inside it, so a user who submits an empty name today may not
  even see the message.
- `Button` supports `.disabled(bool)` through gpui-component's `Disableable`
  trait, and a disabled button does not fire its click handler.
- **Nothing in this app binds Return or Escape to a dialog yet.** The only key
  handling in `crates/knot` is the terminal's `on_key_down` forwarding
  (`workspace_window/mod.rs:3413`) and the app-level `cmd-` bindings in
  `app_bootstrap.rs:246-258`. This change is the first dialog to do it, so it
  sets the pattern the agent editor and the delete confirmation will follow.

## Goals / Non-Goals

**Goals:**

- Return and Escape reach `confirm_workspace_dialog` / `cancel_workspace_dialog`
  while the name field has focus, which is where focus is put when the dialog
  opens (`open_workspace_dialog` calls `input.focus`).
- A mechanism that generalizes: the next modal overlay in this app should be
  able to reuse it rather than invent a second one.

**Non-Goals:**

- Moving the dialog to a real platform sheet or a separate window.
- Relocating or restyling the `self.error` message. Disabling the button
  removes the only route to that message this dialog has; the message stays
  for the "workspace no longer exists" case.
- Global key bindings. These keys must act only while this dialog is open.

## Decisions

### Handle the keys on the overlay, not on the input

The overlay `div` gets a `.on_key_down` listener that matches the keystroke's
`key` against `"enter"` and `"escape"` and calls the existing methods. It does
**not** go through the `Input` entity's own events.

Alternatives considered:

- *Subscribe to an `Input` submit/confirm event.* gpui-component's `Input` in
  0.6 exposes no enter-confirmed event this code can subscribe to, and Escape
  is not an input concern at all. Two different mechanisms for the dialog's
  two keys is worse than one.
- *A gpui action plus `cx.bind_keys`.* Actions are the right shape for
  application commands, and a future `ConfirmDialog` / `CancelDialog` action
  pair with a `key_context` on the overlay is where this ends up if the app
  gains more modals. It is more machinery than one dialog justifies, and
  binding bare `enter` / `escape` actions needs a key context to scope them,
  which needs a focus handle the overlay does not have today. Start with the
  listener; promote it if a second dialog needs it.

Key handling sits on the overlay rather than on the window so it cannot fire
while the dialog is closed - the overlay is only in the tree when
`show_workspace_dialog` is true.

**Risk, to settle in task 1.1 rather than assume:** a focused `Input` may
consume Return and Escape before the event bubbles to the overlay. If it does,
the listener must move to `.on_key_down` with capture semantics, or the design
falls back to the action-plus-`key_context` alternative above. This is the one
unknown in the change and it is checkable in a single run of the app.

### Disable the confirm button rather than validating on press

The confirm button takes `.disabled(name_is_blank)`, computed from the input's
current value at render time.

This is what makes Return's inertness legible. The spec requires Return with a
blank name to do nothing and raise no error; without a visibly disabled
button, "nothing" is indistinguishable from a broken dialog. It also matches
the Swift reference, which disables the button carrying the Return shortcut
(`WorkspaceSheet.swift:99`) rather than guarding inside the action.

The `save_name` empty-name guard stays as defense in depth. It becomes
unreachable through the UI, which is the correct relationship between a
disabled control and the guard behind it - not a reason to delete the guard.

Reading the input's value during render is a read of an entity the view
already holds; it does not need the value mirrored into `WorkspaceManager`
state.

### Return does the same thing in both dialog modes

`confirm_workspace_dialog` already branches on `workspace_dialog_id` to
rename versus create. The key handler calls it unconditionally, so Rename
gets the keys for free. This is intended, not incidental - the spec says so.

## Risks / Trade-offs

- **The `Input` swallows the keys.** → Settled in task 1.1 before the rest of
  the work; the fallback is spelled out above and changes no requirement.
- **A disabled button is a state the user can get stuck in without knowing
  why.** → The field is focused on open and the button is adjacent; an empty
  name in a one-field dialog is self-explanatory. Mitigated further by the
  fact that the previous behavior - a click that silently produced an error
  message rendered behind the overlay - was strictly worse.
- **Escape with unsaved edits discards them without asking.** → Matches the
  reference and matches Cancel, which the same user could already click. A
  workspace name is cheap to retype.
- **This sets a pattern before there is a second case to test it against.** →
  Accepted. The alternative is designing a dialog framework for one dialog.
  The decision above names the promotion path if a second modal needs it.

## Open Questions

None. The one unknown - whether the input swallows the keys - is a task, not
a question: both outcomes are already designed for and neither changes the
spec.
