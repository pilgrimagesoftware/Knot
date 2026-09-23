# Proposal

## Why

The inline permission prompt is already keyboard-operable - `cmd-shift-a`
allows and `cmd-shift-d` denies - but nothing in the prompt says so. The
Allow and Deny buttons render a bare label, so the shortcut is discoverable
only by reading `app_bootstrap.rs`. A user who has to reach for the mouse to
unblock every agent turn is paying the cost the keybindings were added to
remove, because they never learn the keybindings exist.

This is the discoverability half of issue #194, which observed that none of
`permission-prompt-ui` is visible in the running app.

## What Changes

- Show the bound keystroke on the permission prompt's Allow and Deny
  buttons, looked up from the installed keymap rather than hard-coded, so
  the hint cannot drift from the binding that fires.
- Draw the button unchanged when its action has no binding installed, so
  the prompt never promises a shortcut that does nothing.
- Keep the labels, click targets, risk colouring and decision path as they
  are; the hint is additive.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `permission-prompt-ui`: the keyboard-operability requirement gains the
  obligation to show the binding on the control it drives.

## Impact

- `crates/knot/src/panel_view/render.rs`: `render_permission_prompt` gains
  the hint; `render_row` and the list closure in `render_panel` thread the
  `Window` the keymap lookup needs.
- `crates/knot/src/tests/`: a test that the two actions resolve from their
  keystrokes, so dropping a binding fails the build rather than silently
  removing the hint.
- No new dependencies: `gpui_kit::component::kbd::Kbd` already ships the
  lookup and the platform-specific formatting.
