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
from `Agent::markdown_maximized` when the panel opens, which is what finally
gives that field a reader. Seeded, not bound - the user toggling the control must
not write back into agent state that the MCP tool owns, and closing the panel
drops the flag so the next artifact starts from whatever its own call asked for.

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

### The removed focus exceptions are a spec change with no code behind them

Both focus requirements excuse the window from taking focus when a markdown or
diagram pane holds the content area. The code that implements that exception
reads the same agent fields the pane selection reads. Removing the arms from the
pane selection removes the condition; the focus code needs its corresponding
guard deleted, not rewritten. Worth stating because it looks like two changes and
is one.

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
- **The content pane can be squeezed to nothing** → Two panels at 350pt each
  plus a sidebar exceeds a small window. Give the content panel its own minimum
  in the group so the panels yield rather than the conversation vanishing; the
  spec does not name a number for this, so pick one in `consts.rs` and say why.
- **`pane.rs` grows past 700 lines** → It is at 506. The header arm is tens of
  lines, not hundreds, but if it lands over the limit the split is the two pane
  renderers into `pane/markdown.rs` and `pane/mermaid.rs`, not a raised limit.
- **The per-frame file read is now on screen more of the time** → The markdown
  section is visible alongside a conversation that redraws per keystroke, where
  before it replaced that conversation. Same cost per frame, more frames. Not
  fixed here; worth an issue of its own.

## Open Questions

- What minimum the content pane should hold when both side panels are open. It
  is a constant with a comment, decidable during implementation, and no spec
  scenario turns on the number.
