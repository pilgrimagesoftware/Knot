# Tasks

## 1. Virtualized transcript list in the workspace window

- [x] 1.1 Replace the panel's eager `render_panel` scroll content
      (`v_flex().children(messages.map(...))` in `crates/knot/src/panel_view/mod.rs`)
      with a virtualized list so only rows near the viewport are laid out and
      measured. Built on GPUI `List`/`ListState` (via `gpui-kit`) rather than
      `gpui_component::message_scroller::MessageScroller`, whose state exposes
      no way to keep following off at the tail (see design.md). Verify
      `make rust-build`.
- [x] 1.2 Host each panel's `ListState` in the window, not inside the
      `Arc<Mutex<PanelState>>` shared with the ACP reader thread: new
      `panel_lists`/`panel_list_row_counts` maps keyed by agent id replace
      `panel_scroll_handles`. Verify the ACP reader thread still touches only
      `PanelState`. Verify `make rust-build`.
- [x] 1.3 Connect streamed row changes through `ListState::splice` bookkeeping
      (`panel_view::sync_row_count`) so streaming and collapse/expand update
      rows in place without a full conversation rebuild. Keep `PanelState` as
      the single source of truth; the list is a render-side virtualizer only.
      Verify `make rust-build`.

## 2. Virtualization behavior

- [x] 2.1 Streaming splice: reconcile the list's item count to the folded state
      each frame and apply `FollowMode::Tail` while the in-flight response is
      tracked, so the viewport stays at the latest message as it streams;
      `ListState::set_scroll_handler` clears tracking when the user scrolls
      away.
- [x] 2.2 Row content survives virtualization: rows are materialized on demand
      from `PanelState::messages` (the same source the eager build used), and
      `row_at` renders an empty row for an index past the end rather than
      panicking.

## 3. Re-point panel scroll controls onto the virtualized list

- [x] 3.1 Replace the hand-rolled `ScrollHandle`-based action-bar commands and
      the `overflow_y_scroll` container with `ListState` operations:
      scroll-to-user/top via `scroll_to(ListOffset { item_ix, .. })`,
      scroll-to-latest via `scroll_to_end`, and explicit follow mode via
      `set_follow_mode(FollowMode::{Tail,Normal})`.
- [x] 3.2 Delete the per-frame `scroll_to_bottom()`/tracking hack (and
      `SCROLL_BOTTOM_EPSILON`) in the window's dirty-poll render loop; the
      list's follow state replaces it.
- [x] 3.3 Preserve the existing action-bar controls ("scroll to user message",
      "scroll to top", "jump to latest", track toggle) with identical labels,
      icons and placement; give the response action buttons index-qualified ids
      so they stay unique under virtualization.

## 4. Permission prompts and ended banner inside the virtualized list

- [x] 4.1 Move the permission prompt card and the ended-session banner from
      standalone siblings after the message list into virtualized rows of the
      same list, indexed after the messages, so they participate in
      virtualization and tail following.

## 5. Verify and test virtualization

- [x] 5.1 Add regression tests for the row model and reconciliation:
      `row_count`/`row_at` cover messages plus the trailing permission and
      ended rows (and an out-of-range index), and `sync_row_count` grows and
      shrinks a real `ListState`'s item count by the delta. Plus a test pinning
      the `FollowMode::Tail`/`Normal` primitive the reconcile relies on.
- [x] 5.2 Run `make rust-test` / `make rust-fmt` / `make rust-lint` in the
      crate.
