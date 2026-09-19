# Proposal

## Why

The ACP panel renders its entire conversation every frame: `render_panel`
(the `panel_view` module) builds a `v_flex` whose children are **every**
`PanelState::messages` entry, and each assistant row re-parses its full
markdown through `TextView::markdown` on every repaint. Streaming appends a
delta to the last message and marks the session dirty, so each token
re-renders the whole transcript. A long session therefore costs memory and
frame time proportional to the conversation's total length, not to what is
on screen — the panel gets laggy and heavy exactly when it has been used
most. GPUI ships a virtualized `List` element (and gpui-component's
`MessageScroller` built on it) that materializes and measures only the rows
near the viewport; the panel should use one.

## What Changes

- Replace the panel's eager `v_flex(...).children(messages.map(...))` scroll
  content with a GPUI virtualized list, so only the visible rows (plus a
  small overdraw margin) are laid out, measured and painted.
- Adopt a virtual mouse-transcript scroller (`gpui_component::message_scroller`
  `MessageScroller` over GPUI `List`) for the conversation. It provides
  tail-following, splice/remeasure bookkeeping for streaming rows, a
  scroll-to-latest jump button, and a virtual-history scrollbar.
- Move the list's scroll state (`Entity<MessageScrollerState>`) into the
  workspace window's per-session state — it is GPUI-main-thread state and
  cannot live inside the `Arc<Mutex<PanelState>>` shared with the ACP reader
  thread.
- Keep the conversation as one scroller: the pending permission prompt and
  the ended-session banner become trailing rows after the messages rather
  than siblings appended below them.
- Re-point the existing controls at the list: "scroll to user input" and
  "scroll to top" use the list's item scroll; the track toggle and the
  scroll-to-latest button map to its follow-tail mode; scrolling away from
  the bottom pauses following, replacing the manual `ScrollHandle` maths.
- Remove the per-frame `scroll_to_bottom()` hack that paced streaming by
  forcing a scroll on every repaint.
- No visual or interaction behavior changes: message ordering, the
  collapsed/expanded tool cards, the permission prompt, the ended banner and
  the action bars all render as they do today.

**Non-goal:** bounding or truncating retained transcript text. Virtualization
bounds layout/measurement memory and per-frame cost; the `PanelState::messages`
`Vec` and the raw text it holds still grow for the life of a session. Capping
retention changes what history is visible and is a separate decision.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities

- `acp-panel-ui`: the panel's conversation MUST render through a virtualized
  list that materializes only on-screen rows; scroll-to-message, scroll-to-top
  and the follow/track behavior are specified against that list. New
  requirements cover virtualization and its streaming/height-change handling.

## Impact

- `crates/knot/src/panel_view/mod.rs` — `render_panel` becomes a row renderer
  for a virtualized list; `render_message`, the response action bar and the
  track toggle take list scroll state instead of a `ScrollHandle`.
- `crates/knot/src/workspace_window/mod.rs` — `render_panel_pane` hosts the
  per-session list state (replacing `panel_scroll_handles`' `ScrollHandle`),
  drops the hand-rolled overflow container, jump button and scroll maths, and
  drives splice/remeasure from the existing dirty-poll loop.
- `crates/knot/src/panel_state/mod.rs` — no public API change; `messages`
  stays the source of truth. Row-count/height notifications are derived from
  it by the view.
- Dependency: `gpui_component::message_scroller` / GPUI `List` via `gpui-kit`
  (already a dependency; no new crate).
- Tests: panel rendering tests that assert on a rendered element tree may need
  to render through the list (or assert on the row renderer directly).
