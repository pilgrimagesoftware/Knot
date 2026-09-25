# Tasks

## 1. Preferences

- [x] 1.1 Add `KeybindingSettings` and `ShortcutModifiers` to `knot-core`
  settings, `#[serde(default)]`, all fields `Option`. Verify with a round-trip
  test, plus a test that a document without the field, or with an unknown
  sub-field, decodes to defaults.

## 2. Keymap module

- [x] 2.1 Create `crates/knot/src/keymap/` (declarations only in `mod.rs`) with
  the `SelectWorkspace1..9`, `SelectAgent1..9`, `FocusAgentInput`,
  `ToggleDashboard` and `TogglePullRequests` actions and a digit table. Keep
  `OpenCommandCenter` where it is. Verify with `cargo build -p knot`.
- [x] 2.2 Move the fixed bootstrap chords into a `keymap` list that
  `install_actions_and_keys` binds from. Verify with the existing
  `permission_keybindings` and app-menu tests, which must pass unchanged.
- [x] 2.3 Implement `resolve` (stored preferences → chords, default on a parse
  or validation failure) and `validate` (modifier presence, family expansion,
  collisions with fixed and configurable chords). Verify with unit tests for
  every `keybindings` rejection scenario and for the invalid stored binding.
- [x] 2.4 Implement `apply` (Unbind the old chord, bind the new one, remember
  what was applied) and call it from bootstrap in place of the hard-coded ⌥⌘0.
  Verify with a gpui test showing that after a rebind the old chord dispatches
  nothing, the new one dispatches the action, and `bindings_for_action` for
  `OpenCommandCenter` reports the new chord.

## 3. Handlers

- [x] 3.1 Register global handlers for Select workspace N (manager order,
  open-or-raise, no-op out of range). Verify with a test that ⌘2 raises an
  already-open second workspace without opening another window.
- [x] 3.2 Register the handlers for Select agent N, Focus agent input,
  Toggle Dashboard and Toggle Pull Requests. They are global and act on the
  active workspace window; see design.md, Handler placement. Verify with gpui
  tests for the selection, panel-toggle and focus scenarios in `keybindings`,
  including Focus agent input after focus has moved elsewhere in the window
  and from a Dashboard. That test fails without the latch reset.

- [x] 3.3 Add Jump to bottom (default ⌃⌘↓): scroll the selected agent's
  conversation to its end and resume following, as the "Scroll to latest"
  button does. Verify with gpui tests that it scrolls a scrolled-up
  conversation, and leaves it alone while a Dashboard is showing.

## 4. Settings pane

- [x] 4.1 Add `SettingsTab::Keyboard` (eighth tab, label key) and its pane
  height. Verify with a test that `SettingsTab::ALL` ends with Keyboard and
  that the label key resolves.
- [x] 4.2 Build `settings_window/panes/keyboard.rs`: modifier toggles for the
  two families, a recorder per chord, per-row reset and Restore Defaults, and a
  rejection message. Accepted changes write through `settings_global::write`,
  persist, call `keymap::apply`, and rebuild the menus. Verify with gpui tests
  using `with_store_root`: a recorded chord persists and rebinds, Escape
  cancels, and a conflict shows the message and leaves the binding unchanged.
- [x] 4.3 Add all new user-facing strings to `en.yml`, and touch `knot-core` so
  the l10n tests rebuild. Verify with the l10n key-resolution tests.

## 5. Gate

- [x] 5.1 Run `make`. It must pass fmt-check, size-check (no file over 700
  lines), lint, test and build.
- [x] 5.2 Run the app. Check each default shortcut by hand, rebind Open Command
  Center, and confirm that the Window menu shows the new key.
