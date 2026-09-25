# Proposal

## Why

Removing a pull request from the Pull Requests view also opens it in the
browser (#435). `render_row` puts a click handler on the row that opens the URL
and another on the trash control inside it that removes the record; gpui fires
click handlers in the bubble phase, so one click on the control runs both.

The spec did not catch it because it never said the two interact. "A listed
pull request opens in the browser" says a row click opens the URL, and "The
user can remove a recorded pull request" says the view offers removal. Neither
says that using the remove control is not also a row click.

## What Changes

- The remove control stops the click from reaching the row, so removing a
  record no longer opens the pull request.
- The requirement covering removal says so, with a scenario, and says the same
  for any control that may later sit inside one of these rows.
- Tests pin the dispatch order the fix rests on: that gpui bubbles a click from
  a nested control to its ancestors, and that stopping propagation prevents it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `pull-request-tracking`: "The user can remove a recorded pull request" gains
  the requirement that activating the remove control performs only the removal,
  and does not also count as a click on the row it sits in.

## Impact

- `crates/knot/src/workspace_window/render/pull_requests_pane.rs`: one
  `cx.stop_propagation()` in the remove control's click handler.
- `crates/knot/src/tests/pull_request_row_clicks.rs`: new, three tests over a
  probe of the row's nesting.
- No change to `open_pull_request`, `confirm_remove_pull_request`, or anything
  either of them calls. The controls were both correct; only the dispatch
  between them was not.

## Non-Goals

- Making `open_in::open_url` injectable. It shells out to `/usr/bin/open`, so
  the fix cannot be covered by a test that drives the real row's click without
  that test launching a browser on regression. The tests here pin the framework
  behaviour the fix depends on instead, and the gap is stated rather than
  papered over.
- Auditing other panes for the same shape. This is the only control in
  `render/` nested inside a clickable row today; a sweep that finds nothing is
  not worth carrying in this change.
