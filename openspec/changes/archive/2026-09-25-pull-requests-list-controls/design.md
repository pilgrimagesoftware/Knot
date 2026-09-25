# Design

## Context

The Pull Requests view is drawn by `render/pull_requests_pane.rs` (339 lines)
from `WorkspaceWindow::pull_request_groups()` in
`workspace_window/pull_requests_view.rs` (264 lines). That function flattens the
store and the state cache into `PullRequestGroup { agent_ids, agents, rows }`,
each row a `PullRequestRow { url, lookup }`, so no lock is held while elements
are built. Group order and in-group order come from
`pull_request_groups::group_records`.

State arrives through `PullRequestStateCache`, a `RefreshCache<String,
PullRequestLookup>`: `claim_refresh(key, max_age)` hands out a writer only when
the key's last request is older than `max_age`, and the refresh loop skips keys
whose value `is_final()` (not found). The availability probe is gated the same
way by `ForgeStatus::claim_probe`.

The launcher row's breakdown is `pull_request_state::counts_for`, which already
sorts each lookup into open / merged / closed / pending / not found.

The dashboard's `DashboardSort` + `sort_picker` (`dashboard.rs`) is the existing
pattern for a sort dropdown: an enum with a localized `label()`, rendered as a
ghost `Button` with `dropdown_menu`. Row context menus follow
`render/sidebar.rs`'s `.context_menu(...)`. The clipboard is written with
`ClipboardItem::new_string`, as `settings_window/panes/mcp.rs` does.

## Goals / Non-Goals

**Goals:**

- Filtering and sorting as pure functions over the flattened groups, tested
  without GPUI.
- One definition of a row's status category, shared by the launcher counts,
  the status toggles and the bulk removals, so they cannot disagree.
- Bulk removal as one store pass and one settings write.

**Non-Goals:**

- Moving filtering into the store or the cache. Both stay unaware of the view.
- A general-purpose list toolbar component. This is one view's toolbar; the
  dashboard can adopt it later if it needs the same controls.

## Decisions

### View state lives on `WorkspaceWindow`, in one struct

A `PullRequestViewState { sort, statuses, agent, search_input }` field on
`WorkspaceWindow`, alongside the existing `pull_request_states` and
`forge_status`. `statuses` is a `BTreeSet<RowCategory>` (empty means all),
`agent` is `Option<Uuid>`, `sort` is a `PullRequestSort` enum.

The search text is read from the `InputState` entity rather than copied into
the struct, so there is one source of truth for it. The entity is created the
first time the pane renders (it needs a `Window`) and subscribed on `Change` to
`cx.notify()`, so the list narrows per keystroke.

*Alternative:* persisting the view state in settings. Rejected by the proposal's
non-goals, and it would add a settings schema change for no stated need.

### Filtering and sorting in a new pure module

`crates/knot/src/pull_request_filter.rs`, with tests in
`pull_request_filter/tests.rs` - named apart from
`workspace_window/pull_requests_view.rs`, which is the window half - holds:

- `PullRequestSort` - `NewestFirst` (default), `OldestFirst`, `NeedsAttention`,
  `Repository`; closed vocabulary, so an enum with `Display` for its l10n label,
  per the conventions.
- `RowCategory` - `Open`, `Merged`, `Closed`, `NotFound`, `Pending`, with
  `RowCategory::of(Option<&PullRequestLookup>)`.
- `ViewFilter { search, statuses, agent }` and
  `fn apply(groups, &ViewFilter, PullRequestSort) -> Vec<PullRequestGroup>`,
  which drops rows that fail any filter, drops groups left empty, and sorts
  rows within each group.
- `fn category_counts(groups, &ViewFilter) -> BTreeMap<RowCategory, usize>`,
  counting over the rows that pass the search and agent filter but ignoring the
  status filter, which is what each toggle's number means.

`pull_request_state::counts_for` is rewritten on top of `RowCategory::of`, so
the launcher row and the toggles share one classification. Its existing tests
pin that nothing about the launcher changes.

`PullRequestRow` and `PullRequestGroup` move into this module, since it is
their main reader, and the row gains `first_seen` (the earliest across its
agents, from `pull_request_groups::first_seen_by_url`), filled in
`pull_request_groups()` where the store is already locked. Agent names are
matched on the group's heading, which already joins them. Repository and
number are parsed from the URL where they are compared, by a host-agnostic
`repo_and_number` rather than `knot_core`'s scanner, which only recognises its
default hosts: a GitHub Enterprise record still searches and sorts by
repository.

*Alternative:* filtering inside `pull_request_groups()`. Rejected: it mixes a
lock-holding flatten with view logic, and the flatten needs GPUI state to test.

### Needs-attention rank reuses the colour precedence

The rank mirrors `state_color`'s precedence from the spec: conflicting or
blocked, behind, checks running, unknown mergeability, mergeable, then pending,
not found, closed, merged. It is a `fn attention_rank(&PullRequestRow) -> u8`
beside `PullRequestSort`. Ties fall back to newest first with the URL as the
final tiebreak, so the order is total and stable across frames.

### Toolbar in a sibling render file, row menu in the pane

`render/pull_requests_toolbar.rs` draws the search field, status toggles (ghost
buttons, selected ones drawn selected, each with its count), the agent picker
and sort picker (dropdown buttons in the `sort_picker` shape), and the list
actions menu. The toolbar's window state and actions live in
`workspace_window/pull_requests_actions.rs`, beside `pull_requests_view.rs`.

The row context menu is a `.context_menu(...)` on the row in
`render/pull_requests_pane.rs`, applied last so the row's own handlers stay
the row's. (`render/pull_requests_row.rs` is the sidebar's launcher row, not a
per-row helper.) A secondary click does not reach the row's `on_click`, which
is primary-button only; a probe in `tests/pull_request_row_clicks.rs` pins
that. Menu items that open a confirmation defer it with `window.defer`, as the
sidebar menus do, because a popup menu dismisses itself after its handler and
takes an inline dialog with it.

The filtered-empty message and its Clear control are drawn by the pane, since
it is the one that knows whether the unfiltered list was empty.

The agent picker's "return to All agents" rule is enforced where the groups are
computed: if `agent` names no owner of any listed row, it is reset to `None`
before filtering. That covers both a removed agent and one whose last row
expired, with no separate hook on agent removal.

### Bulk removal: one pass, one write

`pull_request_groups::remove_rows(store, rows)` removes each row for each of
its agents and says whether anything went; it is pure over the store, so the
"shown rows only" rule is testable without a window.
`WorkspaceWindow::remove_pull_requests(rows, cx)` calls it with a temporary
store guard, then `persist_pull_requests` and `prune_pull_request_states` once.
The single-row path calls it with one element, so the lock-then-persist
ordering stays in one place.

The rows a bulk action acts on are computed from the same `apply` output the
pane draws, at the moment the menu item is chosen, and captured into the
confirmation dialog's `on_ok`. The dialog body uses `t_with` with the count and,
when a filter or search is active, the shown-only wording.

### Refresh now marks entries stale instead of forgetting them

`RefreshCache::mark_all_stale()` clears the `requested` timestamps and keeps
the values, so every key's next `claim_refresh` succeeds while rows keep
drawing their last state. Forgetting the entries would blank every row, which
the spec forbids.

Not-found rows are skipped by the refresh loop regardless of age. Refresh now
therefore also fills a one-shot `refresh_final: BTreeSet<String>` with the
workspace's URLs. The loop's decision moves into a pure
`pull_request_state::claim_refreshes(cache, urls, asked_again, max_age)`, which
does not skip a URL in that set for finality and takes it out of the set once
its refresh is claimed - and only then, so a fetch already in flight does not
swallow the user's refresh. `ForgeStatus` gains `mark_stale()` for its probe.

Refresh now calls `cx.notify()`, so the next frame runs
`refresh_pull_request_states` and claims the fetches; the answers reach a
frame through the existing `take_changed` link in `repaint_poll_tick`, so no
new link in that chain is needed.

*Alternative:* bypass the cache and spawn fetches directly from the menu
handler. Rejected: it duplicates the claim logic and can race a claim already
in flight.

### Copy URLs writes the shown order

The URLs are joined with `\n` in the order the pane draws them, taken from the
same `apply` output. One clipboard write per action.

## Risks / Trade-offs

- [A fetch in flight when the user chooses Refresh now is claimed again] →
  Two fetches for the same pull request land in either order; both are fresh
  answers, so the row is correct either way. Acceptable for a manual action.
- [Filtering on every frame over the whole list] → The list is tens of rows and
  the work is string comparisons over an already-built snapshot, with no I/O,
  so it is within what the render path is allowed. The search text is
  lowercased once per frame, not once per row.
- [A bulk removal the user did not mean, because a filter was forgotten] → The
  confirmation states the count and, when a filter or search is active, says
  only shown rows are removed.
- [Search input focus stealing terminal keys] → The field is only in the Pull
  Requests view, which replaces the terminal content, so no terminal is
  focused while it is.
- [`counts_for` rewrite changes the launcher row] → Its existing tests run
  unchanged against the new implementation.
