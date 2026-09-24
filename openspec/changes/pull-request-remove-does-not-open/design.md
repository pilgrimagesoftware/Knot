# Design

## Context

See proposal.md - Why. One constraint shapes everything below: `render_row`
builds the row and the remove control as one element tree, and gpui fires click
handlers in the bubble phase (`gpui-pre-0.3.6/src/elements/div.rs:3099`, "Fire
click handlers during the bubble phase"), innermost first. So the control's
handler already runs before the row's; the defect is only that the row's runs at
all.

## Goals / Non-Goals

**Goals:**

- The smallest change that makes the control act for itself alone.
- Tests that fail if the framework behaviour the fix depends on ever changes.

**Non-Goals:**

- Restructuring the row so the two controls are siblings rather than nested.
  Nesting is what makes the trash icon sit inside the row's hover and tint, and
  the spec now covers the interaction directly.
- Covering the fix with a test that drives the real row. See the decision below.

## Decisions

### `cx.stop_propagation()` in the control's handler, before the removal

The codebase's existing idiom for this - `panel/lookup.rs:306` and four more,
`panel/input/entry.rs:79`. Placed before `confirm_remove_pull_request` rather
than after, so the guard does not depend on what that call does or on it
returning at all.

Rejected: a flag on the window read by the row's handler ("a removal was just
confirmed"). That is a second source of truth for something the framework
already models, and it would have to be cleared on a path nothing guarantees.

### The tests pin the framework, not the row

`open_pull_request` calls `open_in::open_url`, which shells out to
`/usr/bin/open`. A test that clicked the real row's remove control and asserted
no browser opened would, on regression, open a browser on the developer's
machine - so the regression it exists to catch is the case where it misbehaves.
That is not a test worth shipping.

What the tests do instead is pin the dependency property the fix rests on, in
the shape `tests/pane_focus.rs`'s dialog probe already uses for gpui's focus
tree: a probe with the same nesting, exercised with and without the guard. The
first test asserts that without it the click reaches the row - that is the
defect, reproduced - and the second that with it the click does not. A gpui
upgrade that stopped bubbling clicks would fail the first and tell the next
reader the guard is now dead weight.

Stated plainly because it is a real gap: deleting `cx.stop_propagation()` from
`render_row` breaks the behaviour and no test fails. Closing it needs
`open_in::open_url` to be injectable, which is a change to a module this bug
does not otherwise touch.

## Risks / Trade-offs

- **The guard is load-bearing and untested at its call site** → The comment at
  the call site says what it is for and what breaks without it, and the tests
  name the file. This is the trade-off above, not a separate risk.
- **Another nested control is added later without the guard** → The requirement
  now covers any control inside one of these rows, rather than the trash icon
  specifically, so the next one has a spec to fail against.
