# Design

## Context

The input area's three Session-Config selectors (permission mode, model,
effort) are rendered by one function,
`WorkspaceWindow::render_panel_config_selector`
(`crates/knot/src/workspace_window/panel/input/controls.rs:150`). The
enabled branch builds a `Popover` whose content is a plain
`v_flex().gap_1().p_1()` of one ghost `Button` per option value
(`controls.rs:205-254`): no height bound, no scroll, no search. The model
selector is where the list is genuinely long - adapters declare the models
an agent can run, and that list commonly exceeds what a dropdown can fit, so
the popover clips at the window edge and later models are unreachable.
See `proposal.md` - Why. The behavioral contract is `acp-panel-ui`'s
"Input area model selector" and the two new requirements in
`specs/acp-panel-ui/spec.md`.

Selection already has a durable, live path this change must preserve,
`controls.rs:235-250`: persist via `remember_session_config` (remembered
session config), then apply to the live session via
`session/set_config_option` (`panel_session` slot's `set_config_option`);
the next message runs under it. The disabled empty state (agent reports no
option) and the permission selector's risk coloring are also in this
function and stay.

gpui-kit 0.6.4 (already pinned) ships a searchable select on top of
`searchable_list` - `gpui_kit::component::select`: a `Select` component
driven by an `Entity<SelectState<D>>`, `searchable(bool)`,
`search_placeholder`, `menu_max_h`, and a `SearchableListDelegate` /
`SearchableListItem` whose default `matches()` is a case-insensitive
substring on the row title. It is not yet used in this crate.

## Goals / Non-Goals

**Goals:**
- The model dropdown scrolls when the declared models overflow the height it
  can occupy, and offers a search field that filters by case-insensitive
  substring (the two new requirements in `specs/acp-panel-ui/spec.md`).
- The existing selection side-effects, the disabled empty state, and the
  permission selector's risk coloring survive unchanged.
- The permission and effort dropdowns inherit the same scroll container via
  the shared render function, with no regression to their short lists.

**Non-Goals:**
- No fuzzy ranking beyond substring matching; no virtualization beyond
  whatever the kit's list does by default.
- No change to `panel_session`, `knot-acp::ConfigOption`, the remembered
  session-config format, or the matching rule that locates the option.
- No change to the agent-type picker in the agent editor (it already scrolls
  via `DropdownMenu::scrollable(true)` through the `agent-persona-picker`
  path) - it is a fixed roster, not the adapter-reported model list.
- The pre-existing unlocalized placeholder/tooltip strings in
  `render_panel_config_selector` are left alone; only new strings this
  change introduces go through `knot_core::l10n::t`.

## Decisions

### 1. Adopt gpui-kit's `Select` instead of extending the hand-rolled popover

Replace the `Popover` of option buttons in the enabled branch of
`render_panel_config_selector` with `Select::new(&state).searchable(true)`,
where `state: Entity<SelectState<ConfigSelectorDelegate>>` backs the option
list. The delegate's item maps one `ConfigOption` value (value + name via
`SearchableListItem::title`) with `Value = String`. `searchable(true)` adds
the search field; the scroll cap comes from `Select`'s `menu_max_h` /
`SearchableList` list behavior; keyboard navigation (up/down/enter/escape)
comes with the component. Because all three selectors render through this
one function, one swap fixes all three.

**Alternative considered - keep the `Popover`, add scroll + a filter input
by hand:** a bounded `overflow_y_scroll` container around the existing
buttons plus an `Input` that narrows `values` on keystroke. Less new
surface, no state plumbing. Rejected: it re-implements what the kit's
searchable list already provides (search field, scroll cap, keyboard
navigation, no-matches handling), and the crate already adopts kit
components (Input, Button, Switch, Tooltip, Popover, GroupBox) rather than
inventing its own. `Select` is the toolkit's intended vehicle for a
searchable, scrollable dropdown.

### 2. SelectState entities live on the window, one per panel and selector

`SelectState` holds focus, scroll position and the search text, so it cannot
be rebuilt per render. `WorkspaceWindow` gains a
`HashMap<(Uuid, SelectorKey), Entity<SelectState<ConfigSelectorDelegate>>>`
keyed by panel id and selector element id (`permission`, `model`,
`effort`), created lazily on first render of that selector and removed when
the panel is removed (same lifecycle as `panel_sessions`). This mirrors how
the agent editor and settings window already hold `InputState` entities they
reuse across renders. When the agent's declared `ConfigOption` changes
between renders, the state entity's delegate items are refreshed (the
`change`/reload path on `SearchableListChange`), so the dropdown never shows
a stale model list.

### 3. Selection wiring is copied, not changed

`SelectEvent::Confirm(Some(value))` runs the exact body the current button
click does (`controls.rs:235-250`): clear `open_config_selector`, persist
with `remember_session_config`, and apply live via `session/set_config_option`
when a session is ready. The trigger label still shows `current_label`; the
"doesn't report selectable models" empty state stays a disabled trigger
button with its tooltip; permission items keep their risk color via
`SearchableListItem::render()` (and the trigger keeps its colored label via
`display_title()`/trigger styling).

### 4. New strings are localized

The search placeholder and the no-matches row are new user-facing text and
go through `knot_core::l10n::t` with keys in
`crates/knot-core/locales/en.yml`, per AGENTS.md (user-facing text is never
hardcoded English).

### 5. Verify with a real-window test, not a unit test

The questions are layout and interaction ("does the dropdown scroll and
search"), which only a window can answer. Follow the existing panel harness
(`crates/knot/src/tests/panel_composer.rs`, `panel_scroll.rs`:
`VisualTestContext` + a real window over `Root`): a panel whose agent
declares a model option with more values than the dropdown can show; assert
(1) default height is bounded and an item below the fold is reachable after
scrolling, (2) typing in the search field filters the listed items and empty
search restores them, (3) confirming a selection reaches
`remember_session_config`/`set_config_option` side-effects. Plus a
few-models case asserting no scroll is needed.

### 6. What implementation changed about decisions 1-3

Three details of the kit's API are not what decisions 1-3 assumed, found
when the code was written against `gpui-component` 0.6.4 (the crate
`gpui-kit` 0.6.4 re-exports as `gpui_kit::component`):

- **`searchable(true)` is on `SelectState`, not on `Select`.** The state is
  built `SelectState::new(delegate, selected, window, cx).searchable(true)`;
  `Select::new(&state)` has no such builder. No behavioural difference.
- **`Select` has no `trigger()`.** It draws its own trigger, so the
  `Button` with the risk-tinted label cannot be handed to it. Only the
  permission selector tints its trigger.
- **`SelectState`'s open flag is private, with no public setter.** Nothing
  outside the kit can open the menu programmatically.

That last one decides which selectors move. ⌘⇧P (`PanelOpenPermissionSelector`,
bound in `app_bootstrap.rs`) opens the *permission* menu by setting
`open_config_selector`, which only the hand-rolled `Popover` reads. Putting
the permission selector on `Select` would take that keyboard path away
silently, and would also cost its trigger tint.

So the swap is narrowed to the two axes that need it and can take it: the
**model** and **effort** selectors render through `Select`
(`panel/input/config_select.rs`), and the **permission** selector keeps the
existing `Popover` path in `controls.rs` unchanged - its list is a handful
of modes, which is the case the popover was already adequate for. Both new
spec requirements are about the model dropdown, so nothing in
`specs/acp-panel-ui/spec.md` changes; what changes is decision 1's "one swap
fixes all three".

`ConfigSelectorDelegate` is `SearchableVec<ConfigSelectorItem>` rather than a
hand-written delegate: a plain `Vec` delegate has no `perform_search`
override and so ignores the query entirely, while `SearchableVec` filters
through `SearchableListItem::matches`, whose default is the case-insensitive
substring the spec asks for. The item type is ours; the filtering is the
kit's.

Refresh (decision 2) is `SelectState::set_items`, not `SearchableListChange`
- the latter is the vocabulary `on_will_change` receives, not a public
setter. `ensure_panel_config_selectors` compares the declared values before
replacing them, so an agent re-reporting the same list does not wipe a
search query mid-typing.

## Risks / Trade-offs

- **[First Select adoption] `Select` is new to this crate; builder details
  (`menu_max_h` interplay with the window edge, focus on open, refresh of a
  changed option) may misbehave in ways the docs don't promise.**
  → Mitigated by the real-window tests in decision 5; if `Select` cannot
  preserve the empty-state/trigger/risk-coloring wiring, fall back to
  decision 1's alternative (scroll container + filter input in the existing
  `Popover`) - a design change, not a spec change.
- **[State plumbing] A third map keyed per panel adds bookkeeping.**
  → Bounded: three fixed slot keys, same create/remove lifecycle as
  `panel_sessions`, and it is what keeps keyboard/search state alive across
  renders; the alternative (stateless) lets every keystroke in the search
  field reset the dropdown.
- **[Stale delegate] The adapter may re-report the model option mid-session;
  the dropdown would keep listing the old models.**
  → Decision 2 refreshes the delegate when the declared `ConfigOption`
  changes, reusing the kit's change mechanism.
- **[Generic storage] A typed `Entity<SelectState<ConfigSelectorDelegate>>`
  map is fine because all three selectors share one delegate type.**
  → If a future selector needs a different value type, it gets its own map;
  not a constraint on this change.

## Migration Plan

Single-function change in
`crates/knot/src/workspace_window/panel/input/controls.rs` plus the l10n
keys, landed through the change's worktree/PR flow. Rollback is reverting
the swap to the `Popover` implementation; no persistence, protocol or
settings format changes to migrate.

## Open Questions

None that would change the specs, the approach, or the task breakdown. The
one real unknown (whether `Select` preserves the existing trigger/empty-state
wiring) is resolved by decision 5's test-first step and has a named fallback.