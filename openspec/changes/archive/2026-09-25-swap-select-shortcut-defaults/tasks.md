# Tasks

## 1. Defaults

- [x] 1.1 In `crates/knot/src/keymap/shortcut.rs`, make `Shortcut::default_modifiers` return ⌥⌘ for `SelectWorkspace` and ⌘ for `SelectAgent`, and replace the Swift-reference comment with one explaining the divergence; verify `cargo build -p knot` succeeds

## 2. Tests

- [x] 2.1 Update `crates/knot/src/keymap/tests.rs` default-label assertions (⌥⌘3 for workspace 3, ⌘2 for agent 2) and the family-collision, digit-collision and workspace-takes-agent-modifier tests so each still exercises a real conflict under the new defaults; verify `cargo test -p knot keymap::` passes
- [x] 2.2 Update `crates/knot/src/tests/keybindings.rs` default and reset assertions (`alt-cmd-N` for `SelectWorkspaceN`, `cmd-N` for `SelectAgentN`); verify `cargo test -p knot tests::keybindings` passes
- [x] 2.3 Update `crates/knot/src/tests/settings_keyboard.rs` `toggling_a_family_modifier_moves_all_nine` for an agent family that starts at ⌘; verify `cargo test -p knot tests::settings_keyboard` passes
- [x] 2.4 Add a `Resolved::from_settings` test for the spec's "Stored family collides with the new default" scenario (stored workspace ⌘, no agent value → both new defaults) and one for "Upgrading with a customized family" (stored agent ⌃⌘ → agent ⌃⌘, workspace ⌥⌘); verify both pass

## 3. Dependent spec text

- [x] 3.1 Update the agent and workspace examples in the `app-menu` View menu requirements (as a delta here); the `sidebar-key-hints` change's `agent-list-ui` delta and design are updated on PR #489; verify `grep -rn "⌥⌘[1-9N]" openspec/specs openspec/changes --exclude-dir=archive` shows no agent-selection example outside the main `keybindings` and `app-menu` specs, which the deltas update at archive

## 4. Verification

- [x] 4.1 Run `make` (fmt-check, size-check, lint, test, build) and verify it passes
- [x] 4.2 In a running build with no customized shortcuts, verify ⌘2 selects the second agent with the terminal focused and with the composer focused, ⌥⌘2 raises the second workspace, and the Keyboard tab shows ⌘ for agents and ⌥⌘ for workspaces
- [x] 4.3 Run `openspec validate swap-select-shortcut-defaults --strict` and verify it passes
