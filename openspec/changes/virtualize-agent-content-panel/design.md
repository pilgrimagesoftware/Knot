# Design

## Context

The panel is `acp-panel-ui`, rendered in the Rust port's workspace window
(`workspace_window::WorkspaceWindow::render_panel_pane` → `panel_view::render_panel`).
Today the conversation is an eager `v_flex` whose children are every
`PanelState::messages` entry, rebuilt each frame. Per-message action bar
controls (track toggle, scroll-to-user, scroll-to-top, jump-to-latest) are
pressed against a manual `scroll`/`ScrollHandle` carved out of the window's
own `on_scroll_wheel`/`overflow_y_scroll` pile, and row anchoring during
streaming is done by `scroll_to_bottom()` calls paced by a per-message
"follow" re-render loop)Skip. See proposal.md - Why.

Constraints that bound this design:

- `PanelState` lives behind `Arc<Mutex<PanelState>>` shared with a background
  reader thread that streams ACP deltas in. GPUI list/scroll state is
  main-thread-only (it centers on `Rc`/`RefCell` and interacts with
  `Window`); it CANNOT be stored in that shared struct.
- The port already depends on `gpui-kit` (a bundled `gpui`, `gpui-component`
  and `gpui-kit-assets`). `gpui-component` ships a ready-made
  `message_scroller::MessageScroller` (a `FollowMode::Tail` aware virtualized
  chat scroller built on GPUI's `List`) plus `scroll_custom_scrollbar` /
  `Button` etc. Reusing those is the fast path; hand-rolling a virtualizer on
  raw GPUI `List` is the fallback.

## Goals / Non-Goals

**Goals**

- Frame cost and retained layout state bounded by viewport size, not
  conversation length (see proposal - Why).
- Preserve every current behavior: streaming in place, tail following,
  per-message and jump controls, collapse/expand, permission prompts,
  ended banner, term/panel toggle.
- Do not change `PanelState`'s public API; keep the ACP reader thread
  touch-point (the streaming splice) talking to the same Arc-shared struct.

**Non-Goals**

- Changing `PanelState::messages` retention/truncation policy (a cap on how
  much raw text is kept is explicitly out of scope here — see proposal
  "Non-Goals").
- Native virtualization for sessions rendered in Terminal mode; only the
  panel path is virtualized.

## Decisions

### Decision: Virtualize with gpui-component's MessageScroller

Use `gpui_component::message_scroller::MessageScroller` (backed by GPUI's
`List` element) as the panel's conversation scroller rather than hand-rolling
a virtualizer.

Rationale:

- It exists and is the exact problem: a chat transcript that streams. It gives
  `FollowMode::Tail` following (auto-scroll to newest output, suspended when
  the user scrolls away — that is precisely the panel's track-toggle
  behavior today), a built-in "jump to latest" button, a virtualized
  "jump to message" scroller for history, `splice`/`remeasure` bookkeeping
  for rows that change height while streaming, and an auto-follow toggle
  that maps 1:1 to the existing track toggle.
- It keeps the port on its existing dependency (`gpui-kit`), so no new crate
  and no new external dependency.

Alternatives considered:

- Hand-rolled virtualization directly on GPUI's `List`/`ListState`. More
  control (e.g. `scroll_to_reveal_item`, custom virtual-history scrollbar)
  but re-implements tail-following, streaming remeasure and the jump button
  that `MessageScroller` already provides — more code to own and test.
- `gpui_component::list` + `ListState` only (no scroller): still must build
  the follow/track and scroll-to-message behavior ourselves.

### Decision: Conversation stays one scroller; extra rows become trailing rows

The virtualized list's row renderer SHALL decide, per index, whether the row
is a message, the permission prompt, or the ended banner — a small enum over
`index < messages.len()` plus the trailing slots — instead of the panel
building sibling elements after the message list.

Rationale: keeps "conversation is one scroll region" (which the proposal and
specs pin: tail-following, scroll-to-message, and the "conversation does not
jump" scenarios all assume one scroller), and lets the permission/ended rows
participate in the same virtualization and follow handling rather than being
pinned siblings that could get out of step with the tail.

### Decision: The scroller state lives on the window's per-session handle, not in PanelState

The `MessageScroller` entity and its list state SHALL be owned by the
per-session `PanelSessionSlot::Ready` handle stored in the workspace
window's `panel_sessions` map (alongside the existing `Arc<Mutex<PanelState>>`)
— not inside `PanelState` itself.

Rationale: GPUI scroller state is main-thread-only and must stay out of the
`Arc<Mutex<>>` shared with the background ACP reader thread. The window
already owns the `panel_scroll_handles`/`FollowHandle` machinery per session;
this consolidates the scroll state in the same place.

### Decision: Streaming drives splice/remeasure, not a full rebuild

When a delta arrives, the session handle's scroller SHALL be told the exact
row range that changed (`splice` for new messages, `remeasure` for a
growing/streaming row) so the list updates in place, mirroring the
`PanelMessage::Assistant`/`ToolCall` streaming appends today. The view
SHALL no longer rebuild `children(messages.map(...))` on every dirty frame.

### Decision: Per-message action bar targets the virtualized row

The existing track-toggle / scroll-to-user / scroll-to-top / jump-to-latest
controls SHALL be re-pointed from the hand-rolled `ScrollHandle` to the
scroller's item-index operations (`scroll_to_top_of_item`, follow mode
toggle, scroll-to-latest), preserving identical user-visible behavior.

## Risks / Trade-offs

- **Streaming row height changes (a growing markdown row) could cause the
  virtualizer to either jump or thrash if not re-measured correctly**
  → Mitigation: use `MessageScroller`'s built-in remeasure/splice for the
  changing row and rely on `FollowMode::Tail` for the tail case; the panel
  already re-renders on each streaming delta, which is when remeasure runs.
- **Virtualized rows whose height depends on streamed content that has
  scrolled out of view could measure differently on the way back**
  → Mitigation: rows are materialized on demand from the same
  `PanelState::messages` source, so state is re-derived identically; the
  spec's "same content and state" scenario guards this.
- **GPUI List rows must report stable, pre-measured heights; permission
  prompts and tool cards that change height (expand/collapse) need a
  remeasure notification**
  → Mitigation: route expand/collapse through the scroller's remeasure for
  that row range (as the tool-call collapse requirement already does).
- **MessageScroller is opinionated about styling; restyling to the panel's
  theme may require wrapping rows in the existing card styles**
  → Mitigation: keep per-row `PanelStyle` wrappers; MessageScroller exposes a
  row overdraw/theme hook. Accept minor styling reconciliation effort.

## Migration Plan

1. Land the virtualized rendering behind the panel's existing view path,
   keeping the Terminal view untouched. No data-model change (messages stay
   the source of truth), so it is a pure rendering replacement.
2. Rollback: the change only replaces the panel's renderer; reverting the PR
   restores the eager `v_flex` path with no migration of persisted state.
3. No schema/persistence changes; no release tagging beyond the normal
   release flow.

## Open Questions

- Whether long-session raw-text retention (capping `PanelState::messages`
  growth) is also wanted. Deferred: resolved after the virtualization
  lands, since it changes conversation-visibility semantics and is
  orthogonal to the renderer change.
