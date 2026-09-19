# Tasks

## Task Group: Virtualized accent content panel

### Task: Adopt a virtual message transcript scroller in the workspace window

Replace the panel's eager `render_panel` scroll content (`v_flex()...
.children(messages.map(...))` in `crates/knot/src/panel_view/mod.rs`) with a
GPUI virtualized message scroller (`gpui_component::MessageScroller` over
GPUI `List`, re-exported via `gpui-kit`'s `component` module), so the panel
materializes and measures only the rows near the viewport.

- Host the scroller's entity state (`MessageScrollerReaction`) in the window's
  per-session slot (`workspace_window::PanelSessionSlot::Ready`) alongside the
  existing `panel_scroll_handles` map — NOT inside the `Arc<Mutex<PanelState>>`
  shared with the ACP reader thread.
- Connect streamed row changes through the scroller's splice/remeasure
  bookkeeping so streaming and collapse/expand update rows in place without a
  full conversation rebuild.
- Keep `PanelState` as the single source of truth for message content/state;
  the scroller is a render-side virtualizer only.

#### Subtask: Message row spliced into the scroller as it streams

Given a visible streaming tail, splice the newly streamed/rematerialized rows
into the message scroller, then confirm the viewport stays at the latest
message while it streams (tracking on) and that scroll-away disables
following (tracking off).

#### Subtask: Row content survives virtualization

Scroll a message far out of the viewport, then scroll back to it, and confirm
the materialized content and state match an eagerly-built conversation.

### Task: Re-point panel scroll controls onto the virtualized list

Replace the hand-rolled `ScrollHandle`-based action-bar commands and the
`overflow_y_scroll` container in `crates/knot/src/panel_view/mod.rs` with the
virtualized scroller's scroll controls: scroll-to-latest (jump + tail follow),
scroll-to-user-input, scroll-to-top, and the built-in follow-mode toggle.

- Delete the per-frame `scroll_to_bottom()`/tracking hack in the window's
  dirty-poll render loop; the scroller's follow/tail state replaces it.
- Preserve the existing action-bar buttons ("scroll to user message",
  "scroll to top", "jump to latest", track toggle) with identical labels,
  icons and placement.

### Task: Keep permission prompts and ended banner inside the virtualized list

Move the permission prompt card and the ended-session banner from standalone
siblings after the message list into virtualized rows of the same scroller,
indexed after the messages, so they participate in virtualization and tail
following.

### Task: Verify and test virtualization

Add a regression test that builds a conversation longer than the viewport,
drives enough scroll/re-ematerialization to reach a far-offset message, and
asserts the row's content and streaming behaviour match the eager baseline.

- Run `make rust-test` / `make rust-fmt` / `make rust-lint` in the crate.
