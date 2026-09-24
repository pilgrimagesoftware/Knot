# Tasks

## 1. Setup

- [ ] 1.1 Create the dedicated worktree + feature branch for this change per AGENTS.md (`<checkout>-Worktrees/model-picker-scroll-search`, branched from `develop`) and verify `git branch --show-current` reports the feature branch, not `develop`/`main`

## 2. Implementation

- [ ] 2.1 Add the l10n keys for the new strings (search placeholder, no-matches row) to `crates/knot-core/locales/en.yml` under the panel namespace, and verify `cargo test -p knot-core` passes (the l10n tests assert keys resolve)
- [ ] 2.2 Define the `ConfigSelectorDelegate` (a `SearchableListDelegate` whose item's `title()` is the option value's `name`, `value()` is its `value`, and default substring `matches()` is used) and its `SearchableListItem`, in or beside `crates/knot/src/workspace_window/panel/input/controls.rs`, and verify `cargo build -p knot` compiles
- [ ] 2.3 Add the per-panel selector state map to `WorkspaceWindow` (`HashMap<(Uuid, SelectorKey), Entity<SelectState<ConfigSelectorDelegate>>>`), created lazily on first render of a selector and removed when the panel is removed, and verify `cargo build -p knot` compiles
- [ ] 2.4 Rewrite the enabled branch of `render_panel_config_selector` (`crates/knot/src/workspace_window/panel/input/controls.rs:205-254`) to render `Select::new(&state).searchable(true)` (placeholder from l10n, `menu_max_h` capping the dropdown) instead of the `Popover` button pile, keeping the trigger label, the colored label for the permission selector, and the disabled empty-state trigger + tooltip, and verify `cargo build -p knot` compiles
- [ ] 2.5 Wire `SelectEvent::Confirm(Some(value))` to the existing selection body (`controls.rs:235-250`): clear `open_config_selector`, `remember_session_config`, then `session/set_config_option` when a session is ready, and verify the click-path test from 2.7 passes
- [ ] 2.6 Refresh the state entity's delegate items when the agent's declared `ConfigOption` changes between renders (via `SearchableListChange`), so the dropdown never shows a stale model list, and verify `cargo build -p knot` compiles

## 3. Tests

- [ ] 3.1 Add a real-window test (following the `crates/knot/src/tests/panel_composer.rs` / `panel_scroll.rs` harness: `VisualTestContext`, real window over `Root`) where the agent declares a model option with more values than the dropdown can display, and verify the dropdown's default height is bounded and a model below the fold becomes reachable by scrolling
- [ ] 3.2 In the same harness, verify typing in the search field filters the listed models by case-insensitive substring, a non-matching query leaves no model selectable, and clearing the query restores the full list
- [ ] 3.3 Verify confirming a selection reaches the `remember_session_config` / `set_config_option` side-effects, and that a short model list (fewer than the dropdown can display) shows every model without scrolling
- [ ] 3.4 Verify `cargo test -p knot` passes the new tests and the existing panel tests (permission/effort selectors, empty state, `panel_scroll`, `panel_composer`) still pass unchanged

## 4. Verification

- [ ] 4.1 Run the full local gate in the worktree (`make fmt`, `make lint`, `make test`, `make size-check`) and verify all pass, with no `.rs` file over the 700-line limit