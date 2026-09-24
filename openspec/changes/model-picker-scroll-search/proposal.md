# Proposal

## Why

The input area's model selector (`panel-model-selector`) renders every model
an adapter declares as a plain vertical pile of buttons inside a `Popover`
(`crates/knot/src/workspace_window/panel/input/controls.rs:205`), with no
height bound and no search field. When an adapter reports a long model list
(claude-code and other CLIs expose tens of models), the popover runs past
the bottom of the window and every model below the fold is clipped and
unreachable; even for lists that fit, finding a model by name means reading
the whole list.

## What Changes

- **The model dropdown scrolls.** When the declared models exceed the
  dropdown's available height, the list becomes a bounded, vertically
  scrollable region, so every model stays reachable by scroll wheel or
  scrollbar - the same intent as the Workspaces list
  (`workspace-manager-scroll`).
- **The model dropdown has a search box.** Typing filters the listed models
  by case-insensitive substring, so a model can be found by name instead of
  by scanning.
- **Selection behavior is unchanged.** Choosing a model still persists
  through the remembered session config and applies to the live session via
  `session/set_config_option`, taking effect from the next message.
- **The empty state is unchanged.** An agent that reports no selectable
  models still shows the disabled "doesn't report selectable models"
  selector. The permission selector's risk coloring is preserved.
- **The permission and effort dropdowns inherit the scroll container.** All
  three selectors render through the same
  `render_panel_config_selector`; fixing the model dropdown fixes the shared
  path, so the other two gain the same scroll cap with no regression to
  their (short) lists.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `acp-panel-ui`: the "Input area model selector" requirement gains
  scrolling for long model lists and a search box that filters by name.
  The requirement's matching rule for locating the option, and the
  change/apply behavior, are unchanged.

## Impact

- `crates/knot/src/workspace_window/panel/input/controls.rs` -
  `render_panel_config_selector`'s popover content is replaced by the
  scrollable, searchable list; the trigger, disabled empty-state, risk
  coloring and the selection side-effects are kept.
- `openspec/specs/acp-panel-ui/spec.md` - the "Input area model selector"
  requirement is extended with scroll and search scenarios.
- `crates/knot-core/src/locales/` - new l10n keys for the search field
  placeholder and the no-matches state.
- No new dependency: gpui-kit 0.6.4 (already pinned) ships the searchable
  select built on `searchable_list`.
- No ACP protocol or data-model changes; the session-config mechanism that
  applies a selection is untouched.