# Proposal

## Why

On macOS the Pull Requests view's search field looks like grey placeholder
text, not a place to type. It has a border, but in the color Knot's system
palette assigns to input borders, `controlBackgroundColor`, which is nearly
the tone of the field's fill and of the toolbar around it. Nothing else in the
toolbar frames the field, so it reads as a label.

## What Changes

- The Pull Requests view's search field draws its unfocused border in the
  separator color instead of the theme's input-border color, so it reads as a
  field in light and dark appearance.
- Focused, it keeps the accent focus border.

## Non-Goals

- Changing the theme's input-border color, or the border of any other text
  field, select or control. Those keep today's appearance.
- Changing the field's fill, size, placeholder or corner radius.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-tracking`: "The user can search the Pull Requests view" gains
  a visible unfocused border on the search field.

## Impact

- `crates/knot/src/workspace_window/render/pull_requests_toolbar.rs` -
  `search_field` sets the field's border color.
- No change to `app_support.rs`'s palette or to gpui-kit.
