# Design

## Context

See `proposal.md` - Why. The state that shapes the approach:

- `WorkspaceWindow::render` (`workspace_window/render/mod.rs`) builds an
  `h_flex` holding two children: the sidebar column, `v_flex().w(px(250.))`,
  and `self.content_column(...)`. The sidebar column contains the window's
  `TitleBar`, so the traffic lights live inside its width; a comment there
  records that the content header is a plain sibling precisely so it starts at
  the sidebar's true right edge with no gutter.
- On macOS `TitleBar` reserves 80px of left padding for the traffic lights, so
  the sidebar's own content (app icon, app name) starts 80px in.
- gpui-kit 0.6 ships `h_resizable(id)`, `resizable_panel()`,
  `ResizableState` and `ResizablePanelEvent::Resized`. A panel takes
  `.size(Pixels)` and `.size_range(Range<Pixels>)`. The group either owns its
  state through `window.use_keyed_state` or binds to an `Entity<ResizableState>`
  the caller holds, and `on_resize` fires on mouse-up, not per drag frame.
- The built-in resize handle is `absolute()`, 1px wide with 4px of padding
  either side, and sets `cursor_col_resize`. It participates in paint but not in
  layout.
- `Settings` is cloned into each window when it opens. `agent_editor` and the
  bench menu both re-read `Settings::load()` before writing, with a comment
  saying why: a window's snapshot goes stale as soon as another window persists.
- Workspace window bounds are persisted per workspace, in the `Workspace`
  record, through a `set_workspace_window_bounds` that returns whether anything
  actually changed so a drag does not write per frame.
- `render/sidebar.rs` is 264 lines and the repo caps a `.rs` file at 700
  (`make size-check`).

## Goals / Non-Goals

**Goals:**

- Use gpui-kit's resizable group rather than hand-rolling a drag, so the cursor,
  the hit area, the clamping and the drag lifecycle come from the toolkit.
- Keep the sidebar/content alignment the existing comment protects.
- One predicate decides compact-versus-full, so the four surfaces that change
  cannot disagree about where the breakpoint is.

**Non-Goals:**

- Generalizing to a reusable Knot-side split-view abstraction. There is one
  divider in the app; a second one (terminal panes) is a different change and can
  reuse the group directly.
- Animating the compact transition. The reference animates sidebar visibility,
  which this change does not implement; the layout swap itself is instant there
  too.

## Decisions

### `h_resizable` with a window-held `ResizableState`

The two columns become the two children of an `h_resizable` group. The sidebar
panel carries `.size(px(settings.sidebar_width))` and
`.size_range(px(SIDEBAR_WIDTH_MIN)..px(SIDEBAR_WIDTH_MAX))`; the content panel
carries no size and takes the remainder.

The state is an `Entity<ResizableState>` the `WorkspaceWindow` owns, not the
group's own keyed state. Two things need to read the width: the render, to
decide compact-versus-full, and the persistence hook. A window-held entity gives
both a single source; `use_keyed_state` would hide it inside the element tree.

Hand-rolling the drag was the alternative - an `on_drag` on a 6px div, as the
Swift reference does with an `NSView`. It would mean reimplementing the cursor
region, the hit padding, the clamp and the drag-in-progress highlight that the
toolkit already has, and getting the layout-neutral positioning right by hand.

### The handle is layout-neutral, which is why alignment survives

The toolkit's handle is absolutely positioned and offset by its own padding, so
it paints over the boundary without taking width from either column. The content
header therefore still starts at the sidebar's right edge, and the comment in
`render/mod.rs` about the gutter stays true. This is worth verifying on the
first build rather than assuming: the header alignment has regressed before.

### Persist on `on_resize`, after re-reading from disk

`ResizablePanelGroup::on_resize` fires on mouse-up with the state entity. The
handler reads `state.sizes()[0]`, then:

1. re-reads `Settings::load()`, falling back to the window's own snapshot;
2. sets `sidebar_width` and persists;
3. writes the value into `self.settings` so this window's next render agrees
   with what is on disk.

Re-reading first follows `agent_editor`'s established pattern - the window's
snapshot is stale the moment another window persists, and writing it back would
silently revert that window's edits. The spec requires exactly this ("A
concurrent settings edit survives").

Persisting on mouse-up rather than per frame keeps file I/O off the render path,
which the project forbids, and matches the intent: the width the user let go of
is the width they chose.

An alternative considered and rejected: per-workspace widths beside
`window_bounds`. Window bounds are per workspace because a window's position is
about that window; a sidebar width is a reading preference that a user would
have to re-set for every workspace. The proposal's non-goals record this.

### Width is adopted at open, not observed while open

The window reads `sidebar_width` when it constructs the panel's size and does not
watch the file. A second window that resizes its own sidebar changes the value on
disk; this window keeps what it has until it is reopened. The spec states this as
a requirement rather than tolerating it as an artifact, because the alternative -
a window resizing itself under the user because another window was dragged - is
worse.

### Clamp on load in `knot-core`, not in the window

`Settings::load_at` brings a persisted width into range, beside the existing
`"SF Mono"` upgrade. Clamping in the window instead would leave the out-of-range
value on disk and oblige every future reader to clamp again. Clamping at the
boundary means a `Settings` value is always renderable, which is what the
`size_range` on the panel then merely re-asserts.

### One `is_compact` predicate, four surfaces

A single function - width against `SIDEBAR_COMPACT_BREAKPOINT` - is evaluated
once per render from the state's current width and threaded to the four surfaces
that change: the agent row, the dashboard row, the sidebar's title bar and the
new-agent button. A `bool` parameter on each, not a second render path for the
whole sidebar: the row's click handling, context menu, selection background and
not-running dimming are identical in both layouts, and duplicating the row
wholesale is how the two would drift.

The compact agent row lives in a new sibling module
(`render/sidebar_compact.rs`) rather than inside `render/sidebar.rs`, which is
already 264 lines and would approach the 700-line cap with a second row layout
inlined.

### The state dot moves onto the avatar when compact

At full width the dot is a third column beside the text block. With the text
hidden there is no such column, so the compact row overlays the dot on the
avatar's bottom-trailing corner, as `AgentRowView.compactBody` does. The dot
keeps its colour mapping and its "shell agents have none" rule, so the only
change is where it sits.

### The minimum is 120px, not the reference's 80px

The Swift sidebar starts below the title bar; the Rust sidebar column contains
it, and `TitleBar` reserves 80px for the traffic lights. At 80px the sidebar
would be traffic lights and nothing else, and the app icon would be clipped.
120px leaves the icon visible and fits the compact row's 40px avatar with its
padding. The deviation is deliberate and recorded in the spec.

## Risks / Trade-offs

- [The resizable group changes how the sidebar column's `TitleBar` and the
  content header line up, reintroducing the gutter the existing comment warns
  about] → First task after wiring the group is to check that alignment at
  several widths; the handle's absolute positioning says it should hold, but the
  comment exists because this broke before.
- [`ResizableState` redistributes sizes when the container resizes, so resizing
  the *window* could change the sidebar's width] → Verify by resizing the window
  at a non-default sidebar width; if the sidebar drifts, pin it by re-applying
  `.size(settings.sidebar_width)` each render and letting only the drag change
  it.
- [Persisting from the workspace window is a new responsibility for a window
  that until now only wrote through the agent store, so a stale-snapshot write
  could clobber unrelated settings] → The re-read-then-write sequence is the
  mitigation, and the spec pins it with a scenario.
- [Two open workspace windows disagree about the width, which reads as a bug
  even though it is specified] → Accepted: the alternative resizes a window the
  user is not touching. Stated in the spec so it is a decision, not a surprise.
- [The compact layout hides the agent name, and a tooltip is the only way back
  to it] → Matches the reference. The breakpoint is 160px, well below the 250px
  default, so a user only reaches it by dragging there deliberately.
- [Hiding the app-name label in the sidebar title bar at narrow widths could
  clip the traffic lights anyway on a future title-bar change] → The 120px floor
  is derived from the 80px the toolkit reserves; if that constant changes
  upstream the floor needs revisiting, which the constant's doc comment says.
