# Tasks

## 1. Reproduce and pin the dispatch

- [x] 1.1 Add `crates/knot/src/tests/pull_request_row_clicks.rs` with a probe of
      the row's nesting, and a test asserting that without the guard a click on
      the inner control also reaches the row - the defect, reproduced. Verify it
      passes, which is what proves gpui bubbles the click.
- [x] 1.2 Add the companion test asserting the guard keeps the click from the
      row, and a third that the row still takes clicks outside the control, so
      the fix cannot pass by disabling the row. Verify all three pass.

## 2. Fix

- [x] 2.1 Call `cx.stop_propagation()` at the top of the remove control's click
      handler in `workspace_window/render/pull_requests_pane.rs`, before
      `confirm_remove_pull_request`; comment it with what breaks without it.
      Verify `make lint` and `make test` pass.

## 3. Contract

- [x] 3.1 Modify `pull-request-tracking`'s "The user can remove a recorded pull
      request" to require that activating the control performs only the removal,
      phrased for any control inside one of these rows rather than the trash icon
      alone, with scenarios for removing-does-not-open and for the row still
      opening beside it. Verify `openspec validate --strict` passes.

## 4. Gate

- [x] 4.1 Run `make` and confirm the full gate passes.
