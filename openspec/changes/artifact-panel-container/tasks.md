# Tasks

## 1. Layout arithmetic

- [x] 1.1 Create `crates/knot/src/workspace_window/artifact_panel/` with a
      `mod.rs` that declares `layout`, `state` and `render` and holds the module
      doc linking back to `openspec/specs/artifact-panel/spec.md`; verify
      `make lint` passes and `mod.rs` holds no implementation.
- [x] 1.2 Add the section-height constants to `crates/knot/src/consts.rs`:
      collapsed header height, divider height, default/min/max panel width
      (500/350/800), split clamp (0.15..=0.85), default split 0.5, and the
      content pane's minimum width for the unexpanded case, with a comment
      saying it does not apply while the panel is expanded; verify
      `make build` passes.
- [x] 1.3 Implement `layout::section_heights(total, split_ratio,
      markdown_collapsed, mermaid_collapsed) -> (f32, f32)` in `layout.rs`,
      porting the four cases from `ArtifactPanelView.sectionHeight`; verify with
      tests in `layout/tests.rs` covering both expanded at the default and at
      0.7, each section collapsed alone, and both collapsed.
- [x] 1.4 Add the invariant test: for every ratio in 0.15, 0.3, 0.5, 0.7, 0.85
      the two heights plus the divider height equal the total; verify it passes
      as the Swift suite's `bothExpandedSumToTotal` does.

## 2. Per-agent panel state

- [x] 2.1 Add the per-agent maps to `WorkspaceWindow` in
      `workspace_window/window.rs` - panel width, split ratio, markdown and
      mermaid collapse flags, expanded flag - initialized empty at
      `workspace_window/open.rs`; verify `make build` passes.
- [x] 2.2 Implement the accessors in `artifact_panel/state.rs`: readers that fall
      back to the constants from 1.2 and writers that clamp width and ratio;
      verify with tests asserting an unset agent reads the defaults and an
      out-of-range write is clamped at both ends.
- [ ] 2.3 Add `artifact_panel_open(id)` reading `markdown_file` and
      `mermaid_source` from the store, and re-seed the expanded flag from
      `Agent::markdown_maximized` whenever that agent's
      `(markdown_file, markdown_maximized)` pair differs from the pair last
      seeded for it - keyed on the pair, not the file alone, which would
      strand a re-show of the open file asking to be maximized, and not on
      every call, which would collapse a hand-expanded panel when an agent
      re-shows a file it edited.
      Verify with three tests covering the `mcp-tools` scenarios "A maximized
      file reaches an already-open panel" and "Re-showing a file leaves the
      user's panel alone", plus a file change carrying a different argument.
- [x] 2.4 Drop every per-agent entry when an agent closes, alongside
      `git_panel_width` in `workspace_window/git_panel/reads.rs`; verify with a
      test that closing a resized agent leaves no entry behind, covering the
      spec's "Closing an agent discards its arrangement".
- [x] 2.5 Clear the expanded flag when the last section closes, and leave it
      alone when one section closes while the other stays open. Note the trap:
      `clear_markdown_panel` (`knot-agents/src/store/panels.rs:24`) is a third
      writer of `markdown_maximized`, setting it false whenever the markdown
      file closes, so the reset must test that no diagram remains rather than
      following that field - Swift's guard is the `else if mermaidSource ==
      nil` at `ContentView.swift:118`. Verify with tests covering "Expanded
      state does not carry to the next artifact" and "Closing one section does
      not collapse the panel", the second with an expanded panel whose markdown
      section is closed while a diagram is still open.
- [x] 2.6 Make the expand control write the window flag only, never
      `Agent::markdown_maximized`; verify with a test that toggling it leaves
      the agent's stored panel state untouched.

## 3. Collapsible section headers

- [ ] 3.1 Give `render_markdown_pane` and `render_mermaid_pane` in
      `workspace_window/panel/pane.rs` a parameter carrying whether a collapse
      control is drawn and whether the section is collapsed; verify existing
      callers compile and `make test` still passes.
- [ ] 3.2 Draw the chevron in each section's header when collapsible, toggling
      that section's flag, and render the header alone when collapsed; verify
      with a render test asserting a collapsed section's body element is absent
      and its header present.
- [ ] 3.2a Keep each section's own close control in its header alongside the
      chevron, closing only that section's artifact; verify with a test
      covering "Closing one section leaves the other".
- [ ] 3.3 Add the localization keys for the panel title, the expand and collapse
      tooltips, the close-all tooltip and the chevron's label to
      `crates/knot-core/src/l10n/en.yml`, and replace the literal
      `.tooltip("Close")` on the diagram section's close button
      (`workspace_window/panel/pane.rs:100`) with the `panel.close` key the
      markdown section already uses; touch `knot-core` afterwards per the
      stale-artifact hazard. Verify with tests asserting each key resolves,
      never the English copy, and grep the module for remaining string
      literals in tooltips.

## 4. The panel container

- [ ] 4.1 Implement the toolbar in `artifact_panel/render.rs` - localized title,
      expand toggle, close-all - with close-all clearing both the markdown and
      mermaid panel state through the store; verify with a test that close-all
      on an agent with both artifacts leaves it with neither.
- [ ] 4.2 Render the single-section case: the one open section filling the panel,
      no divider, no collapse control; verify with a render test covering the
      spec's "Only a diagram is open".
- [ ] 4.3 Render the dual-section case with `v_resizable`, feeding the two
      panels the heights from `layout::section_heights` and writing the clamped
      ratio back on `on_resize`; verify by dragging the divider in the running
      app and observing the two sections trade height within the clamp.
- [ ] 4.4 Render the collapsed cases as a plain `v_flex` with no resizable group
      and no divider; verify with a render test asserting no divider element
      when either section is collapsed.

## 5. Wiring into the content area

- [ ] 5.0 Add an artifact-state landing to `repaint_poll_tick`'s `if` chain
      (`workspace_window/repaint.rs:126-144`): a per-agent snapshot of
      `markdown_file`, `mermaid_source` and `markdown_maximized` compared
      against what was last drawn, true when it differs. The MCP tools write
      the store from another thread with no context to notify from
      (`knot-mcp-tools/src/panels.rs`), and nothing in the chain reads those
      fields today. Take the comparison into a local rather than into the `||`
      chain, so a short-circuit cannot strand it. Verify with a test that an
      artifact set for an idle, non-streaming agent marks the window for
      repaint, covering "An artifact for an idle agent still appears".

- [ ] 5.1 Remove the markdown and diagram arms from the pane selection in
      `workspace_window/render/content.rs` so the session pane is chosen for an
      agent with artifacts open; verify the conversation is drawn for an agent
      with a markdown file open.
- [ ] 5.2 Replace `with_git_panel` with a `with_side_panels` that adds whichever
      of the git and artifact panels are open as further `resizable_panel`s in
      the one `h_resizable` group, in the order content, git, artifact, with the
      content panel carrying the minimum from 1.2; verify `make test` passes and
      the git panel still resizes as before.
- [ ] 5.3 Wire the panel's width through the group's `on_resize` to the clamped
      writer from 2.2, and hide the drag handle while the panel is expanded;
      verify by resizing in the running app, expanding, and returning to the
      same width.
- [ ] 5.4 Give the content panel the minimum width from 1.2 as its `size_range`
      floor for the unexpanded case, and collapse it to no width, hidden and
      not hit-testable while the panel is expanded, leaving the sidebar and any
      open git panel alone; verify with tests covering "An expanded panel hides
      the content pane" and that an unexpanded panel cannot drag the content
      pane below its minimum.
- [ ] 5.5 Verify both panels open together in the running app: the content pane,
      then the git panel, then the artifact panel, each draggable, covering the
      spec's "Both panels open".

## 6. Focus guards

- [ ] 6.1 Remove `has_markdown` and `has_diagram` from
      `pane_focus::SelectedAgentFacts` and delete the
      `if agent.has_markdown || agent.has_diagram` guard at
      `workspace_window/pane_focus.rs:88` outright - nothing replaces it there,
      and the two facts stop being read in `render/mod.rs:195-206`. One guard
      serves both focus paths, so this is a single edit. Verify `make build`
      passes.
- [ ] 6.1a Add the expanded check inside `focus_showing_pane`, beside the
      `window.has_active_dialog(cx)` guard (`render/mod.rs:227-229`) - after
      `self.focused_pane = showing`, before focus is taken - reading the
      window's live expanded flag, not `Agent::markdown_maximized`. Putting it
      in `focus_target` instead would latch `None` while expanded and take
      focus on the `None -> Composer` transition that collapsing produces.
      Verify with a test that a collapse following a click elsewhere leaves
      focus on the clicked control, covering "Collapsing an expanded panel does
      not pull focus" in both deltas. Add the condition inside
      `focus_showing_pane` only, never to a call before it: everything
      `prepare_frame` runs ahead of that builds or reconciles state and must
      keep running while expanded, or state absent on the collapse frame draws
      as an empty-state placeholder.
- [ ] 6.2 Update `crates/knot/src/tests/pane_focus.rs` - including
      `an_open_markdown_file_shows_no_composer` at :216, whose premise this
      change reverses - and add four cases: a Panel-mode agent with a markdown
      file open in an unexpanded panel focuses its composer, the same agent
      expanded leaves focus alone, and the Terminal-mode pair of the same.
      Verify they cover both modified `acp-panel-ui` scenarios and both
      modified `terminal-input` scenarios. Note that the expanded cases can no
      longer be exercised through `focus_target` alone, since the check moved
      to `prepare_frame` - they need the window-level test path the dialog
      guard already uses.

## 7. Integration and gate

- [ ] 7.1 Add a test that `view-mermaid` on an agent with a markdown file open
      leaves both artifacts set and neither tool reporting the other closed,
      covering the modified `mcp-tools` scenario.
- [ ] 7.2 Verify the whole flow in the running app: have an agent call
      `display-markdown` then `view-mermaid`, confirm both sections appear
      beside a readable conversation, collapse each in turn, drag both handles,
      expand and close.
- [ ] 7.3 Run `make` and confirm the full gate passes, including
      `make size-check` on `pane.rs` and every new file.
- [ ] 7.4 Verify the dashboard case in the running app: with an agent's
      markdown file open, switch the window to its dashboard and confirm the
      artifact panel stays shown beside it, covering "The dashboard does not
      close the panel".
