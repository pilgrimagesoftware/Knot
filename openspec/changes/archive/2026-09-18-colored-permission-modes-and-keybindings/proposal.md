## Why

The permission-mode selector and the inline permission prompt in the agent
panel (`crates/knot/src/main.rs`, `crates/knot/src/panel_view.rs`) render as
plain, uncolored controls today. A user granting `bypassPermissions` (or an
equivalent unrestricted mode) gets no visual warning that they're disabling
guardrails, and every permission decision - allow, deny, or a mode switch -
requires a mouse click, with no keyboard path. This is a usability and safety
gap: destructive-capable modes should read as visually distinct from safe
ones, and a prompt that blocks the agent's turn should be dismissible without
reaching for the mouse.

## What Changes

- Color-code the permission-mode selector button and its dropdown items by
  risk level (e.g. bypass/dangerous modes in red/warning color, restricted
  "plan"-style modes in a neutral/safe color, default mode unstyled).
- Color-code the inline permission prompt's border/accent to match the
  requested mode's risk level where the agent reports one, falling back to
  the current neutral blue border when no mode context is available.
- Add keybindings for the inline permission prompt: allow and deny can be
  triggered from the keyboard while the prompt is focused/visible, without
  a mouse click.
- Add a keybinding to open the permission-mode selector's dropdown from the
  keyboard while the panel input area has focus.
- Keep mouse interaction fully working; keybindings are additive.

## Capabilities

### New Capabilities
- `permission-prompt-ui`: color-coding and keyboard interaction for the
  agent panel's permission-mode selector and inline permission prompt.

### Modified Capabilities
(none - no existing spec currently documents this UI)

## Impact

- `crates/knot/src/main.rs`: `render_panel_config_selector` (mode selector
  styling + dropdown keybinding), `actions!` block and key-binding
  registration, `render_panel_input_area`.
- `crates/knot/src/panel_view.rs`: `render_permission_prompt` (border color
  by risk level, allow/deny keybindings).
- `crates/knot/src/panel_state.rs`: no change expected unless risk-level
  needs deriving from `PermissionRequest`/`ConfigOption` data already
  present.
- No new dependencies; uses gpui-kit's existing `KeyBinding`/`actions!` and
  `Button` color APIs.
