# Tasks

## 1. Name the items that need a key

- [x] 1.1 Give each standard menu item that macOS gives a key equivalent,
      and that has no behavior yet, its own action in `app_bootstrap.rs`.
      They cannot share `NoAction`: a menu item's shortcut is looked up by
      action, so shared items would all show the same key, and a `NoAction`
      binding is treated by `Keymap::add_bindings` as a *disabling* binding
      and would unbind that key everywhere. Mark the block `UNWIRED`.
- [x] 1.2 Leave Zoom on `NoAction`. It needs no name, because macOS gives it
      no key equivalent.

## 2. The bindings

- [x] 2.1 Bind ⌘N, ⌘W, ⌃⌘F, ⌘M and ⌘? to the actions from 1.1.
- [x] 2.2 Point the Edit menu's five items at `gpui_kit::base::input`'s
      `Undo`, `Redo`, `Cut`, `Copy` and `Paste` rather than at placeholders,
      and add no bindings of our own for those keys - gpui binds them in its
      `Input` key context already, and a context-less binding of ours would
      out-rank it and break copying in every text field.
- [x] 2.3 Drop `.disabled(true)` from the Edit items: those five do work,
      and AppKit enables them from `is_action_available` on the focused
      element's dispatch path.

## 3. Tests

- [x] 3.1 Add a test that each standard shortcut resolves to its item's
      action, driving the real keymap through `install_actions_and_keys`.
      Done as `standard_menu_items_carry_their_platform_shortcut`.
- [x] 3.2 Add a test that ⌘Z, ⇧⌘Z, ⌘X, ⌘C and ⌘V still resolve to gpui's
      text actions after our bindings are installed, so a placeholder added
      on one of those keys fails loudly rather than silently breaking text
      fields. Done as `the_edit_menu_leaves_the_text_keys_with_gpui`.

## 4. Verification

- [x] 4.1 Run `make` and verify fmt, size-check, clippy, tests and build all
      pass for the workspace.
- [ ] 4.2 Open the app and walk every menu, confirming each item listed in
      the spec's table shows its key - greyed on the items with no behavior
      yet - and that About Knot, Show All, Zoom and the Agents items show
      none.
- [ ] 4.3 In the running app, confirm Edit > Copy enables with a settings
      text field focused and copies the selection, and that ⌘C in a terminal
      pane still copies the terminal's selection.
