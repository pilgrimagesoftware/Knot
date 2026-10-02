# Design

## Context

The Workspaces window renders its content in one column: a title bar, a
toolbar (`v_flex().justify_end()` with the command-center and New Workspace
buttons), then the workspace rows as a plain `v_flex().gap_2().children(rows)`
inside `v_flex().flex_1().gap_4().p_4()` (`crates/knot/src/workspace_manager/
render.rs:164`). That rows container has no height bound and no
`overflow_y_scroll`, so with enough workspaces the rows run past the window
and are clipped. See `proposal.md` for the motivation and
`openspec/specs/workspace-manager-ui/spec.md` for the contract.

The codebase already has the exact pattern this needs in the settings window:
a scroll region that fills whatever height the surrounding flex column leaves
it, without a hard-coded height:

```rust
let mut settings_body = div().id("settings-body").flex_1();
settings_body = settings_body.overflow_y_scroll();
// …
.child(settings_body.child(body))
```

(`crates/knot/src/settings_window/render.rs:78-91`, with the personas tab
pinning a header above the scrolling list at
`settings_window/panes/personas.rs:214`.) `overflow_y_scroll()` is the
idiom used in a dozen places across the crate - `processes_pane.rs:295`,
`mcp_pane.rs:273`, `pull_requests_pane.rs:51`.

## Goals / Non-Goals

**Goals:**
- The workspace list scrolls vertically when its rows overflow the window,
  keeping every workspace reachable (the new requirement in
  `specs/workspace-manager-ui/spec.md`).
- Title bar, toolbar and the error banner stay pinned; only the rows move.
- The change is local to the manager window's render and introduces no new
  dependency.

**Non-Goals:**
- No change to rows themselves - their actions, drag-reorder, double-click
  to open, or selection highlighting are untouched.
- No virtualization or lazy loading; the manager lists workspaces, which are
  few in practice. This is a scroll **container**, not a `List`.
- No change to the workspace agent sidebar (`agent-list-ui`) or any other
  window.

## Decisions

### 1. Bound the list with flex, not a fixed height

Wrap the rows in `div().id("workspace-list").flex_1().overflow_y_scroll()`
inside the existing content `v_flex`. `flex_1` sizes the container to the
height the toolbar leaves, so the scroll region is exactly as tall as the
window allows; `overflow_y_scroll()` is what makes wheel and scrollbar reach
the rest.

This is the settings window's body. It was chosen over the processes pane's
`.max_h(px(BODY_MAX_HEIGHT))` (`processes_pane.rs:298`), which caps a list at
a constant height regardless of window size - right for a fixed-height
collapsible section, wrong for a window whose content area is the whole
remainder of the window.

### 2. Where the scroll container sits relative to the chrome

Structure change in `render.rs` only: the toolbar `h_flex` and the error
banner stay direct children of the padded content `v_flex` (pinned); the
scroll container replaces the current `.child(v_flex().gap_2().
children(rows))` as their sibling:

```
v_flex().flex_1().gap_4().p_4()
  ├─ h_flex (toolbar: command center, New Workspace)   // pinned
  ├─ div#workspace-list .flex_1().overflow_y_scroll()
  │    └─ v_flex().gap_2().children(rows)              // scrolls
  └─ error banner (div().child(error))                 // pinned
```

The `gap_4` spacing between scroll region and its siblings is preserved by
the flex column. The list's bottom rows scroll under the column's `p_4`
bottom padding; the selections, buttons and drag handles a row carries go
with it because they all live on the row block, moved as one unit.

### 3. Reuse the existing build loop, rows unchanged

`render.rs` already builds every row with `Self::workspace_row` in a `for`
loop before returning the element tree; that loop, the `rows` vector and
`workspace_row` are untouched. Only the container those rows are pushed into
changes. The rows keep their `workspace-row-{id}` ids, so drag targets and
the dialogs they open are unaffected.

### 4. Verify with a real-window render test

Follow the `tests/workspace_dialog.rs` harness: a real window over a
`VisualTestContext` with a throwaway temp settings root, a `WorkspaceManager`
seeded with many workspaces (a size chosen so the rows clearly exceed a
short window), then assert that scrolling the list makes the bottom-most
row's region reachable - either by dispatching a scroll and checking the
scroll offset changes, or by asserting the row block intersects the window
content bounds after a scroll. This mirrors how `panel_scroll.rs` probes
scroll behavior for the same reason: the question is about layout, which only
a window can answer.

## Risks / Trade-offs

- **[Flex min-height] A `flex_1` child's implicit min-height can defeat the
  scroll** - if the flex implementation sizes shorter than the content, the
  container overflows instead of scrolling. The settings window scroll region
  works without a `min_h(0)`, so the default already behaves, but if the
  short-window test shows overflow, add `.min_h(px(0.))` to the container
  (the utility exists; `workspace_window/render/mod.rs:371`).** →
  Decided at implementation; the render test in decision 4 is what tells us
  which.
- **[Scrollbar overlay] A scrollbar drawing over the last row's buttons** -
  the trailing rename/delete buttons plus drag handle sit at the rows' right
  edge. If gpui-kit draws the scrollbar inline rather than overlayed, the
  rightmost controls of the bottom row are partially covered while
  scrolled.** → Cosmetic; buttons remain clickable at their left portion and
  rows are 28px-tall enough that no action becomes unreachable. Not worth an
  inset unless a user reports it.
- **[Scrolling the wheel over the list] A row's own drag/click handlers still
  receive clicks; scroll-wheel is not consumed by rows**, so no conflict with
  the row actions or drag-reorder. `overflow_y_scroll` owns wheel input only
  on the container's empty chrome.

## Migration Plan

Single-file render change in `crates/knot/src/workspace_manager/render.rs`,
landed as part of the change's existing commit/worktree flow. Revert is a
one-line diff back to the unscrolled container; no data or settings change to
roll back.

## Open Questions

None that would change the spec, approach or task breakdown.