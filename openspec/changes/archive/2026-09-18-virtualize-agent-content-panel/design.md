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
"follow" re-render loop. See proposal.md - Why.

Constraints that bound this design:

- `PanelState` lives behind `Arc<Mutex<PanelState>>` shared with a background
  reader thread that streams ACP deltas in. GPUI list/scroll state is
  main-thread-only (it centers on `Rc`/`RefCell` and interacts with
  `Window`); it CANNOT be stored in that shared struct.
- The port already depends on `gpui-kit` (a bundled `gpui`, `gpui-component`
  and `gpui-kit-assets`). `gpui-component` ships a ready-made
  `message_scroller::MessageScroller` (a `FollowMode::Tail` aware virtualized
  chat scroller built on GPUI's `List`), but its state exposes no API to keep
  tail-following *off* while the view is at the bottom (`set_follow_mode` /
  `pause_following_tail` are not public on `MessageScrollerState`), which the
  track toggle requires. GPUI's own `List`/`ListState` exposes exactly those
  primitives, so the panel hand-rolls on it; see the decision below.

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

### Decision: Virtualize on raw GPUI `List`, not `MessageScroller`

Build the panel's conversation scroller directly on GPUI's `List`/`ListState`
(re-exported by `gpui-kit`) rather than on `gpui_component`'s
`MessageScroller`.

Rationale:

- The track toggle has a hard requirement `MessageScroller` cannot meet: when
  the user turns following *off*, the list must stay put **even at the tail**.
  `MessageScrollerState` exposes no `set_follow_mode`/`pause_following_tail`,
  so its `Tail` mode re-engages the moment the view returns to the bottom,
  silently overriding the toggle. `ListState` exposes `set_follow_mode`,
  `is_following_tail` and `pause_following_tail` publicly, which is the whole
  mechanism the panel needs.
- Everything else `MessageScroller` would have provided — `splice` for new
  rows, `scroll_to`/`scroll_to_reveal_item` for the action bar, `scroll_to_end`
  for "jump to latest", `set_scroll_handler` for detecting scroll-away — is on
  `ListState`. The extra code is the small, testable follow-mode reconciliation
  in `render_panel_pane`, not a re-implemented virtualizer.
- It stays on the existing `gpui-kit` dependency, so no new crate.

Alternatives considered:

- `gpui_component::message_scroller::MessageScroller`: less code, but fails the
  track-toggle-off-at-tail requirement (above), and the toggle behavior is
  contractual (see `acp-panel-ui`'s track toggle scenarios).
- `gpui_component::list` wrapper only: same missing follow-mode API as
  `MessageScroller`; would still need the raw `ListState` underneath.

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

### Decision: The list state lives on the window, not in PanelState

Each panel's `ListState` SHALL be owned by the workspace window — in the
`panel_lists` map keyed by agent id, alongside a `panel_list_row_counts` map
holding the count each list was last reconciled to — and NOT inside
`PanelState` itself.

Rationale: GPUI list state is main-thread-only (`Rc`/`RefCell`-based) and must
stay out of the `Arc<Mutex<>>` shared with the background ACP reader thread.
The window already owns the other per-agent view state; this replaces the
per-session `panel_scroll_handles` machinery in the same place. The map is
created lazily and dropped with the session (`remove_session` /
`retry_panel_session`), so a reconnect rebuilds the list — and its
`set_scroll_handler`, which captured the old session slot.

### Decision: Streaming drives splice, not a full rebuild

Each frame the pane is rendered, the list's item count SHALL be reconciled to
the folded state by splicing only the delta (`PanelState::messages.len()` plus
the optional permission row and ended row; see `panel_view::sync_row_count`),
so off-screen rows keep the heights they were already measured at. A *visible*
row whose content grew (a streaming `PanelMessage::Assistant`) needs no splice:
GPUI's list re-renders and re-measures visible rows every layout pass, which
`ListState::scroll` already triggers on each streamed delta. The view SHALL no
longer rebuild `children(messages.map(...))` on every dirty frame.

### Decision: Per-message action bar targets the list's item indices

The existing track-toggle / scroll-to-user / scroll-to-top / jump-to-latest
controls SHALL be re-pointed from the hand-rolled `ScrollHandle` to `ListState`
item-index operations: `scroll_to(ListOffset { item_ix, .. })` for
scroll-to-user/top (which also stops tail-following), `scroll_to_end()` for
"jump to latest", and `set_follow_mode(Tail | Normal)` for the track toggle,
preserving identical user-visible behavior. Button ids carry the row index so
they stay unique under virtualization.

## Risks / Trade-offs

- **Streaming row height changes (a growing markdown row) could cause the
  virtualizer to either jump or thrash if not re-measured correctly**
  → Mitigation: the list re-measures visible rows every layout pass, and the
  panel re-renders on each streaming delta, which is when that pass runs;
  tail-following (`FollowMode::Tail`) keeps the growing row anchored to the
  bottom.
- **The follow mode must not clobber the user's scroll position or the track
  toggle** → Mitigation: `FollowMode::Tail` is applied only on the transition
  into tracking (guarded by `is_following_tail`), and `FollowMode::Normal` is
  re-asserted while tracking is off, so a toggled-off list stays put even at
  the tail; a user scroll clears tracking via `set_scroll_handler`.
- **Virtualized rows whose height depends on streamed content that has
  scrolled out of view could measure differently on the way back**
  → Mitigation: rows are materialized on demand from the same
  `PanelState::messages` source, so state is re-derived identically; the
  spec's "same content and state" scenario guards this.
- **GPUI list rows must report stable heights; permission prompts and tool
  cards that change height (expand/collapse) need a fresh layout pass**
  → Mitigation: expand/collapse changes the row's rendered content, and the
  dirty-poll re-render runs a layout pass that re-measures it.
- **A row index the list still holds for one frame (after a `splice` lands but
  before the next reconcile) could be out of range** → Mitigation:
  `panel_view::row_at` returns `None` past the end and the row renders empty,
  so a racing frame cannot panic.

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
