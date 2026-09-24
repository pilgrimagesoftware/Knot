# Design

## Context

See proposal.md - Why for the motivation. The constraints that shape the
approach:

- `crates/knot/src/workspace_window/render/content.rs` picks one pane for the
  content area from an ordered list: markdown file, then diagram, then the
  stopped placeholder, then the panel or terminal session. The markdown and
  diagram arms are the two this change removes from that list.
- The git panel is the precedent for a side panel. `with_git_panel`
  (`content.rs:311-354`) wraps the chosen pane in an `h_resizable` group with
  the panel as the second `resizable_panel`, sized from a per-agent width map on
  `WorkspaceWindow` and clamped both by the panel's `size_range` and again in
  `set_git_panel_width`. The width is in-memory only and is dropped when the
  agent closes.
- `gpui_kit::component::resizable` exports `h_resizable`, `v_resizable`,
  `resizable_panel` and `ResizableState`. Only the horizontal form is used in
  the tree today - the sidebar/content split and the git panel split. The
  section divider is the first vertical use.
- `Agent` already carries every piece of artifact state this needs:
  `markdown_file`, `markdown_maximized`, `markdown_history`, `mermaid_source`,
  `mermaid_title` (`crates/knot-agents/src/agent.rs:101-106`).
  `markdown_maximized` is written by `display-markdown` and by the close path and
  is read by no UI code at all.
- `pane.rs` is 506 lines against a hard 700-line limit, and `mod.rs` files may
  not hold implementation.
- No I/O on the render path. `render_markdown_pane` already breaks this - it
  calls `std::fs::read_to_string` every frame - and this change does not fix
  that, but it must not make it worse by reading in the container as well.

## Goals / Non-Goals

**Goals:**

- One container module owning the panel's layout and its per-agent arrangement.
- The two existing pane renderers reused as sections, changed only by gaining a
  collapsible header.
- Section-height arithmetic in a pure function with its own tests, the way the
  Swift reference's `sectionHeight` is a static function with its own suite.

**Non-Goals:**

- Fixing `render_markdown_pane`'s per-frame file read. It is a real defect and it
  predates this change; fixing it here would mean adding a watched cache, which
  is its own change.
- A general artifact registry. Two sections, named, is what the Swift reference
  has and what the store's fields support.
- Animating the panel's appearance or its collapses.

## Decisions

### The panel is a second resizable sibling, not a nested split

`with_git_panel` already produces an `h_resizable` group when the git panel is
open. The artifact panel becomes a third `resizable_panel` in that same group
rather than a group nested inside the content panel.

One group means one drag model: each handle moves the boundary it sits on and
the group reconciles the rest, so dragging the git panel's edge with the
artifact panel open does what the user expects. Nesting would make the outer
drag resize a subtree containing a panel with its own fixed width, and the
inner panel would absorb or refuse the change depending on which side was
dragged.

Consequence: `with_git_panel` becomes a `with_side_panels` that takes the pane
and adds whichever of the two panels are open, in the order content, git,
artifact. Rejected alternative: leaving `with_git_panel` alone and wrapping its
result. That reads as the smaller diff and produces the nested group above.

### Arrangement lives on `WorkspaceWindow`, keyed by agent, not in settings

Four per-agent maps beside `git_panel_width`: width, split ratio, collapse flags
for the two sections, expanded flag. Cleared on agent close through the same
path `git_panel_width` uses.

`WorkspaceUiState` was the alternative and is the wrong shape twice over: it is
keyed by workspace while all of this is per agent, and adopting it would make the
artifact panel the only side panel whose width survives a relaunch while the git
panel's does not. See proposal.md - Deliberately not carried over.

The expanded flag is the one exception to "starts at a default": it is seeded
from `Agent::markdown_maximized`, which is what finally gives that field a
reader. When and how often it is re-seeded is its own decision below.

### Section heights come from a pure function, not from flex

`section_heights(total, split_ratio, markdown_collapsed, mermaid_collapsed) ->
(f32, f32)` in the container module, ported from Swift's
`ArtifactPanelView.sectionHeight` and its four cases: both collapsed, either
collapsed, both expanded. It returns both heights at once rather than answering
per section, because the invariant worth testing is that the two plus the divider
equal the total - the Swift suite asserts exactly that across five ratios
(`SkwadTests/Views/Artifacts/ArtifactPanelLayoutTests.swift`).

Flex weights were the alternative. They cannot express the collapsed cases: a
collapsed section is a fixed header height and the other section takes the
remainder, which is a different rule from a ratio, and encoding both in flex
means a weight that changes meaning with the collapse flags.

### `v_resizable` drives the divider, with the ratio derived from its sizes

The divider uses `v_resizable` with the same `on_resize` shape the git panel
uses: read the group's sizes, convert the first to a ratio of the total, clamp to
0.15..=0.85, store it. The pure function above still computes the heights fed to
the two panels, so the clamp is enforced in one place and the collapsed cases
never reach the resizable group at all - when either section is collapsed the
container renders a plain `v_flex` with no group and no handle.

Risk noted below: `v_resizable` has no user in this tree yet.

### The container is a new module; the panes stay where they are

`crates/knot/src/workspace_window/artifact_panel/` with the module's own
declarations in `mod.rs` and implementation in siblings - `layout.rs` for the
pure height function and its tests, `render.rs` for the panel and toolbar,
`state.rs` for the per-agent maps' accessors.

`render_markdown_pane` and `render_mermaid_pane` stay in `pane.rs` and gain a
parameter for the collapsible header: whether a collapse control is drawn, and
whether the section is collapsed. Moving them would make the diff a move plus a
change and hide both. `pane.rs` at 506 lines has room for the header arm; the
container's ~300 lines would not fit and are why it is a new module.

### Expanded is a window-level flag seeded from the agent's field, not a copy

`Agent::markdown_maximized` is written unconditionally by every
`set_markdown_panel` call and reset by `clear_markdown_panel`
(`crates/knot-agents/src/store/panels.rs:15,24`), and `convert.rs:47` defaults it
false on load, so it is already session-scoped. It is the *command channel*: what
an agent asked for. The window's per-agent expanded flag is the *live state*:
what is on screen, including what the user toggled.

The edge between them is one-way, and its trigger is the `(file, maximized)`
pair rather than the file alone: the window remembers the pair it last seeded
from and re-seeds when either half differs. Swift triggers on the file alone
(`ContentView.swift:113-117` seeds `artifactExpanded` from `markdownMaximized`
inside `.onChange(of: activeAgent?.markdownFilePath)`, which does not fire for
an equal value), and that has a hole the port should not inherit: re-showing the
file already open, this time asking for it maximized, changes nothing.

Triggering on every call was the alternative. It closes the same hole but opens
another — an agent re-showing a file it has just edited, with `maximized`
omitted as it always was, would collapse a panel the user had expanded by hand.
It also needs a per-call signal the store does not currently carry, since
`set_markdown_panel` writes the same value twice for a repeated call. The pair
comparison needs no new state on the agent record and no new field in the store.

`clear_markdown_panel` is a third writer of `markdown_maximized`
(`store/panels.rs:24`, setting it false when the markdown file closes) and is a
trap for this design: a re-seed that fired on the file becoming `None` would
collapse a panel still showing a diagram. Swift guards it with the `else if
activeAgent?.mermaidSource == nil` at `ContentView.swift:118`. Here the guard is
inherent — `None` is not a file, so it is not a pair to seed from — but the
reset on panel close has to check the diagram itself.

Seeding once when the panel first opens was the alternative and is wrong: an
agent that shows one file without `maximized` and then another with it would
never expand, because the seed had already run. Binding the control directly to
the agent field was the other alternative and is also wrong: the user's toggle
would then be indistinguishable from an instruction the agent gave, and would be
written into state the MCP tool owns.

### Expanded guards taking focus; it is not an input to the latch

`prepare_frame` latches the whole answer, `None` included
(`render/mod.rs:213-214`: `let changed = showing != self.focused_pane;`). Putting
the expanded state into `focus_target` would therefore latch `None` while
expanded and produce a `None -> Composer(id)` transition on collapse — which
takes focus, from wherever the user had put it, on an action that is not a change
of selection.

So expanded does not join `focus_target`'s branch order. `focus_target` keeps
answering what the *selection* implies, and the expanded check sits beside the
dialog guard in `prepare_frame`, after the latch is stored and before focus is
taken. That is the pattern the dialog case already uses, and for the same
reason: the latch has to record that the selection was seen, so that the state
clearing later is not replayed as news.

Consequence for `SelectedAgentFacts`: `has_markdown` and `has_diagram` come off
it and nothing replaces them. The expanded flag is window state, which
`prepare_frame` has in hand, so it never becomes a fact read out of the store.
That also removes the ordering question against `has_live_grid` — expanded no
longer participates in the ordering at all.

The predicate reads the window's live expanded flag rather than
`Agent::markdown_maximized`. The two diverge the moment the user collapses a
panel an agent maximized, and it is what is on screen that decides whether
there is a composer to focus.

The expanded condition belongs to this guard and to no other, which is worth
saying because `prepare_frame` is becoming where per-frame preamble collects.
It already holds `refresh_terminal_font`; PR #431 moved
`reconcile_panel_send_chord` in beside it, and PR #424 adds the model and effort
dropdown state. Every one of those builds or reconciles state, and none of them
may be skipped while the panel is expanded.

The dropdown block is the clearest case of why. Skipping it while expanded would
leave that state absent on the frame the panel collapses, where
`render_panel_searchable_selector` falls back to a disabled empty-state trigger
— so the user would see both selectors drawn greyed-out for one frame before
they populate. Building it while expanded costs a map lookup and a small vector
comparison and draws nothing.

The distinction is whether focus may be *taken* versus whether state *exists*.
Only the first is a question about what is on screen. Making the neighbouring
gates symmetrical with this one would be a regression, not a tidy-up.

### Both focus deltas are one edit, not two

`acp-panel-ui` and `terminal-input` each carry the exception, but one guard
serves both paths — `focus_target` answers for the composer and the terminal
surface from the same branch chain, which is why it is one function rather than
two predicates with a latch each (`pane_focus.rs` module doc). Deleting the
artifact arm and adding the expanded guard above therefore satisfies both
deltas at once. Worth stating because the two requirements read as two pieces
of work.

## Risks / Trade-offs

- **`v_resizable` is unexercised in this tree** → Build the divider behind the
  pure height function, which is independently testable, so a `v_resizable` that
  misbehaves costs the drag gesture and not the layout. If it cannot be made to
  work, the fallback is a hand-rolled drag on a 4pt `div` with a mouse-move
  handler, which is what the Swift reference does; the stored ratio and the
  height function are unchanged either way.
- **Three panels in one resizable group may not reconcile as expected** →
  Verify with both panels open before wiring the second drag handle. If the group
  fights itself, the fallback is to give the artifact panel a fixed `size_range`
  whose min and max are both the stored width, making it rigid while the git
  panel's handle moves, at the cost of one frame of lag on its own drag.
- **The content pane can be squeezed to nothing while unexpanded** → Two panels
  at 350pt each plus a sidebar exceeds a small window. Give the content panel a
  `size_range` minimum so the panels yield rather than the conversation
  vanishing by accident. Expand is the deliberate case and overrides it: the
  content pane goes to zero width only when the panel is expanded, which is the
  one state both focus deltas already account for.
- **`pane.rs` grows past 700 lines** → It is at 506. The header arm is tens of
  lines, not hundreds, but if it lands over the limit the split is the two pane
  renderers into `pane/markdown.rs` and `pane/mermaid.rs`, not a raised limit.
- **The per-frame file read is now on screen more of the time** → The markdown
  section is visible alongside a conversation that redraws per keystroke, where
  before it replaced that conversation. Same cost per frame, more frames. Not
  fixed here; worth an issue of its own.

## Open Questions

None.

Two readings of the Swift reference were corrected during review and are
recorded here so they are not re-derived. `ArtifactPanelView.swift:55`
(`maxWidth: isExpanded ? .infinity : nil`) is only half of expand; the other half
is `ContentView.swift:232-234`, which gives the terminal area `width: 0`,
`opacity: 0` and `allowsHitTesting(false)` while expanded. Swift's expand is a
takeover, not a squeeze. And `ArtifactPanelView` does not read the agent's
`markdownMaximized`, but `ContentView.swift:116` does, which is where the
seeding lives.
