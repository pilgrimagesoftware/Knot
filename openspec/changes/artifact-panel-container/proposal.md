# Proposal

## Why

An agent can put two things in front of the user — a markdown file through
`display-markdown` and a diagram through `view-mermaid` — but the Rust port has
no container to hold them. `render_markdown_pane` and `render_mermaid_pane` are
both wired into `content.rs` as content-area takeovers, tried in that order, so
an agent that shows a diagram while a markdown file is open produces no visible
change: the markdown pane wins and the diagram is stored where nothing draws it.
The Swift reference puts both in one side panel with a draggable divider.

The takeover has a second cost. While either pane is open the conversation and
its composer are gone, which is why `acp-panel-ui` and `terminal-input` each
carry an exception excusing the window from focusing an input that is not on
screen. A side panel removes the condition those exceptions were written for.

## What Changes

- A new artifact panel: a trailing side panel, sibling to the content pane,
  holding the markdown section and the mermaid section. It is shown whenever the
  selected agent has a markdown file or a diagram open, and closes when both are.
- The two panes stop taking over the content area whenever an artifact is open.
  The conversation or terminal narrows beside the panel instead, and is replaced
  only while the user or an agent has expanded the panel. **BREAKING** for the
  two focus requirements that treat any open markdown or diagram pane as a
  reason not to focus an input: the exception narrows to the expanded case
  rather than disappearing.
- A draggable vertical handle sets panel width, clamped to the same 350–800pt
  range the git panel uses, per agent.
- A toolbar over both sections carrying the panel's title, an expand toggle that
  gives the panel the whole content area and back, and a close-all control. Each
  section also keeps its own close control, and the diagram section's — literal
  English today — gets localized with the rest.
- When both sections have content, a draggable horizontal divider between them
  sets the split, clamped to 0.15–0.85, and each section's header gains a
  collapse chevron. A collapsed section shows its header only and yields its
  height to the other; both collapsed shows two headers.
- With only one section active it fills the panel, with no divider and no
  chevron — the single-section case has nothing to split or trade space with.
- `Agent::markdown_maximized`, written by `display-markdown`'s `maximized`
  argument and read by nothing since it was ported, drives the panel's expanded
  state — re-read each time the agent's markdown file changes, as the Swift
  reference does it (`ContentView.swift:113-117`). An agent asking for a
  maximized file gets one, on the first call and on every later one.

### Deliberately not carried over

- **Animated transitions.** The Swift panel slides in from the trailing edge and
  animates every collapse with `withAnimation(.easeInOut(0.2))`. The state
  changes are ported; the animation is not. Nothing else in the Rust port
  animates a layout change, and adding the first one here is a larger decision
  than this change should make.
- **A window-wide expanded flag.** Swift holds `artifactExpanded` on the
  workspace view, so expanding the panel for one agent expands it for the next
  agent the user selects. Expanded state is per agent here, matching the panel
  width and the underlying `markdown_maximized` field, which is per agent too.
- **Persistence of any of it.** Panel width, split ratio, collapse and expanded
  state live in memory on the workspace window and are gone when it closes. The
  git panel's width already works this way, Swift's `@State` does not survive
  either, and `WorkspaceUiState` is keyed by workspace while all of this state is
  per agent. Persisting it would make the artifact panel the only side panel
  whose arrangement outlives its window; that is a change worth proposing on its
  own evidence, not a rider on this one.
- **Reworking either pane.** The markdown pane's review controls (approve,
  comment, submit) exist in Swift's `MarkdownPanelView` and not in the Rust port.
  They stay unported. This change adds a container and a collapsible header to
  the panes it already has.

## Capabilities

### New Capabilities
- `artifact-panel`: the container that holds an agent's markdown file and
  diagram together — when the panel is shown, where it sits relative to the
  content pane and the git panel, its width and expand toggle, the divider and
  per-section collapse between the two sections, and what happens as sections
  open and close.

### Modified Capabilities
- `acp-panel-ui`: the requirement "Selecting a Panel-mode agent focuses its
  prompt input" excuses the window from taking focus when "an open markdown or
  diagram pane holding the content area ahead of the conversation" is showing.
  That pane no longer holds the content area, so the exception is removed and the
  composer is focused as it is for any other selected Panel-mode agent.
- `terminal-input`: the same exception, in the same words, in "Selecting a
  Terminal-mode agent focuses its terminal surface". Removed for the same reason.
- `mcp-tools`: `display-markdown`'s `maximized` argument is specified as updating
  panel state but nothing says what it does. It sets the artifact panel's
  expanded state for that agent.

## Impact

- `crates/knot/src/workspace_window/render/content.rs`: the pane selection stops
  returning the markdown and mermaid panes ahead of the session panes, and the
  artifact panel joins the git panel as a second resizable sibling.
- `crates/knot/src/workspace_window/panel/pane.rs`: `render_markdown_pane` and
  `render_mermaid_pane` become sections with collapsible headers rather than
  full-height panes. The file is 506 lines and the 700-line limit is close, so
  the container lands in its own module.
- `crates/knot/src/workspace_window/window.rs`: per-agent maps for panel width,
  split ratio, collapse flags and expanded state, cleared with the agent as
  `git_panel_width` already is.
- `gpui_kit::component::resizable`: `h_resizable` for the panel width, as the git
  panel and the sidebar already use it, and `v_resizable` for the divider — the
  first vertical use in the tree.
- `knot-core::l10n`: keys for the panel title, the expand and collapse tooltips,
  the close-all tooltip, and the section chevrons.
- No change to `knot-agents`, the MCP tools, or the settings store. Every field
  this needs is already there.
