# Proposal

Issue: #479

## Why

The Pull Requests view lists every pull request a workspace's agents have
opened, in one fixed order, and the only thing the user can do to the list is
remove one row at a time. A long-running workspace accumulates dozens of rows:
finding the one pull request that conflicts means reading all of them, and
cleaning out closed or not-found rows means one confirmation dialog per row.

## What Changes

- A toolbar at the top of the Pull Requests view, above the groups.
- **Search:** a text field that narrows the list to rows whose title, number,
  repository, URL or opening agent's name contains the typed text.
- **Filter by status:** toggles for open, merged, closed, not found and
  pending - the launcher row's own breakdown, so a draft counts as open - each
  showing how many rows it would add. Selecting none shows everything.
- **Filter by agent:** a picker that narrows the list to one agent's pull
  requests, including the ones it shares with other agents.
- **Sort:** newest first (today's order and the default), oldest first, needs
  attention first, and repository then number. Sorting orders rows within each
  agent group; the grouping itself is unchanged.
- **List actions:** a menu with Refresh now, Copy URLs, Remove merged, Remove
  closed, Remove not found and Remove all. Copying and removing act on the rows
  currently shown, so with no search or filter Remove all clears the list.
  Every removal asks for confirmation, states how many rows it removes, and
  affects only Knot's records.
- **Row actions:** a context menu on each row with Open in browser, Copy URL
  and Remove, so the per-row actions are reachable without aiming at the
  trash icon.
- An empty result caused by the search or filters says so, with a control that
  clears them, instead of claiming the workspace has no pull requests.
- **Refresh now** also asks again about pull requests previously found not to
  exist, which today requires a restart.

The launcher row's counts are unchanged: they describe the workspace, not the
current search or filter.

## Non-Goals

- Persisting the search text, filters or sort order across a restart. They
  belong to one window's session with the view.
- Acting on GitHub: nothing here merges, closes, approves or comments on a pull
  request.
- Opening several pull requests in the browser at once.
- A keyboard shortcut to focus the search field. It needs a keymap entry that
  does not collide with terminal find, which is a separate decision.
- Changing how rows are grouped by agent.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-tracking`: adds search, status and agent filters, sort orders,
  list-level and row-level actions to the Pull Requests view; newest first
  becomes the default order rather than the only one; a manual refresh re-asks
  pull requests found not to exist.

## Impact

- `crates/knot/src/workspace_window/render/` - the Pull Requests pane gains a
  toolbar, a filtered-empty state and a row context menu. New sibling files
  keep `pull_requests_pane.rs` under the 700-line limit.
- `crates/knot/src/workspace_window/pull_requests_view.rs` - bulk removal and
  manual refresh; the view state for search, filters and sort lives on
  `WorkspaceWindow`.
- A new pure module for filtering and sorting rows, unit-tested without GPUI.
- `crates/knot/src/refresh_cache.rs` - a way to mark every entry stale while
  keeping its value, so a manual refresh does not blank rows.
- `crates/knot-core/locales/en.yml` - new `pull_requests.*` keys.
- No change to the persisted record format, `knot-forge` or `knot-agents`.
