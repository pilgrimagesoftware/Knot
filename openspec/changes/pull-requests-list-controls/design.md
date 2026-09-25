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

`PullRequestRow` gains the fields filtering and sorting need, filled in
`pull_request_groups()` where the store is already locked: `first_seen` (the
earliest across the row's agents), `agent_names`, and the parsed
`PullRequestUrl` (owner, repo, number) so a row without state can still be
searched and sorted by repository. Parsing reuses
`knot_core::pull_request_url`; a recorded URL is canonical, so it always
parses, and a row that somehow does not falls back to matching on the raw URL.

*Alternative:* filtering inside `pull_request_groups()`. Rejected: it mixes a
lock-holding flatten with view logic, and the flatten needs GPUI state to test.

### Needs-attention rank reuses the colour precedence

The rank mirrors `state_color`'s precedence from the spec: conflicting or
blocked, behind, checks running, unknown mergeability, mergeable, then pending,
not found, closed, merged. It is a `fn attention_rank(&PullRequestRow) -> u8`
beside `PullRequestSort`. Ties fall back to newest first with the URL as the
final tiebreak, so the order is total and stable across frames.

### Toolbar and row menu in sibling render files

`render/pull_requests_toolbar.rs` draws the search field, status toggles (ghost
buttons, selected ones with the theme's selected background, each with its
count), the agent picker and sort picker (dropdown buttons in the
`sort_picker` shape), and the list actions menu. `render/pull_requests_row.rs`
already exists for row helpers; the context menu goes there. This keeps
`pull_requests_pane.rs` under the size limit and leaves it composing parts.

The filtered-empty message and its Clear control are drawn by the pane, since
it is the one that knows whether the unfiltered list was empty.

The agent picker's "return to All agents" rule is enforced where the groups are
computed: if `agent` names no owner of any listed row, it is reset to `None`
before filtering. That covers both a removed agent and one whose last row
expired, with no separate hook on agent removal.

### Bulk removal: one pass, one write

`remove_pull_requests(&mut self, rows: &[(Vec<Uuid>, String)], cx)` replaces the
single-row `remove_pull_request`'s body: it takes the store lock once, removes
each row for each of its agents, drops the lock, then calls
`persist_pull_requests` and `prune_pull_request_states` once. The single-row
path calls it with one element, so the lock-then-persist ordering the existing
comments warn about stays in one place.

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
therefore also sets a one-shot `refresh_final: BTreeSet<String>` of the
workspace's URLs; the loop does not skip a URL in that set for finality, and
removes it once its refresh is claimed. `ForgeStatus` gains the same
`mark_stale()` for its probe.

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
