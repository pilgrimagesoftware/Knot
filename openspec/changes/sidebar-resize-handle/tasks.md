# Tasks

## 1. The stored width (`knot-core`)

- [x] 1.1 Add `SIDEBAR_WIDTH_DEFAULT` (250.0), `SIDEBAR_WIDTH_MIN` (120.0), `SIDEBAR_WIDTH_MAX` (400.0) and `SIDEBAR_COMPACT_BREAKPOINT` (160.0) to `crates/knot-core/src/consts.rs`, the minimum's doc comment deriving 120 from the 80px the title bar reserves for the traffic lights; verify `make lint` reports no unused constant once 1.2 and 3.x reference them.
- [x] 1.2 Add `sidebar_width: f64` to `Settings` defaulting to `SIDEBAR_WIDTH_DEFAULT`, and clamp it into `SIDEBAR_WIDTH_MIN..=SIDEBAR_WIDTH_MAX` in `Settings::load_at` beside the existing `"SF Mono"` upgrade; verify with tests for a fresh store, a document without the key, an in-range value, a value of `40`, a value of `5000`, and a non-numeric value, per the scenarios in `specs/settings-persistence/spec.md`.
- [x] 1.3 Add a write-then-reload test in `crates/knot-core/tests/settings.rs` covering the last scenario (set `180.0`, persist, reload, read back `180.0`); verify `make test` passes.

## 2. The divider (`knot`)

- [x] 2.1 Give `WorkspaceWindow` a `sidebar_resize: Entity<ResizableState>` field with a doc comment saying why the window owns it rather than the element tree, and construct it in `workspace_window/open.rs`; verify the workspace window still opens and renders unchanged.
- [x] 2.2 In `workspace_window/render/mod.rs`, replace the `h_flex` holding the two columns with `h_resizable` bound to that state - the sidebar as a panel sized from `settings.sidebar_width` with `size_range` from the two bounds, the content column as an unsized panel - keeping the sidebar column's own `v_flex` contents unchanged; verify the window renders, the divider draws, and the pointer over it becomes the column-resize cursor.
- [ ] 2.3 Verify the alignment the comment in `render/mod.rs` protects: at 120px, 250px and 400px the content column's header starts at the sidebar's right edge with no gutter, and the traffic lights are unclipped. Update that comment to mention the divider if the reasoning has moved.
- [ ] 2.4 Verify dragging against the bounds: dragging right past 400px stops at 400px, dragging left past 120px stops at 120px and never hides the sidebar, and the divider is visibly active while dragged.
- [ ] 2.5 Resize the window at a non-default sidebar width and verify the sidebar keeps its width rather than being redistributed; if it drifts, re-apply the panel size each render so only a drag changes it, per `design.md` - Risks.

## 3. Persisting a finished drag

- [x] 3.1 Add the `on_resize` handler on the group: read the sidebar panel's size, re-read `Settings::load()` (falling back to the window's snapshot), write `sidebar_width`, persist, and update `self.settings` so the next render agrees with disk; verify by dragging, closing the workspace window, reopening it, and finding the sidebar at the dragged width.
- [ ] 3.2 Verify the handler runs once per drag rather than per frame - it is wired to the group's resize-finished callback, not to pointer movement - by confirming `settings.json`'s modification time changes once across one drag.
- [ ] 3.3 Verify a concurrent edit survives: change a setting in the settings window, then drag a workspace window's divider, and confirm both the dragged width and the other setting are in effect after a reload.
- [ ] 3.4 Verify the two multi-window scenarios: a newly opened second workspace window adopts the dragged width, and an already-open second window keeps the width it has.

## 3b. The terminal pane's geometry (not in the original plan)

- [x] 3b.1 `workspace_window/mod.rs` held `TERMINAL_SIDEBAR_WIDTH: f32 = 250.`, the hard-coded left edge `resize_session_to_pane` and `grid_position` both measure the terminal pane from. A draggable sidebar makes it wrong the moment it is dragged - the PTY grid would be sized for a 250px sidebar and every mouse position would be offset by the difference. Both call sites now read `WorkspaceWindow::sidebar_width`, and the constant is gone.
- [ ] 3b.2 Verify against the running app: drag the sidebar wide and narrow with a terminal agent selected, and confirm the grid reflows to the pane and a click lands on the cell under the pointer (e.g. in `vim` or a shell with mouse reporting).

## 4. The compact layout

- [x] 4.1 Add the compact predicate (current sidebar width against `SIDEBAR_COMPACT_BREAKPOINT`), evaluated once in `render/mod.rs` from the resize state and threaded to the sidebar surfaces as a `bool`; verify it is the only place the breakpoint is read.
- [x] 4.2 Add `crates/knot/src/workspace_window/render/sidebar_compact.rs` rendering the compact agent row - avatar centered, state dot overlaid on the avatar's bottom-trailing corner (and absent for a shell agent), name as the row's tooltip, no detail lines - reusing the full-width row's selection background, not-running dimming, companion indent, click handler and context menu; verify `make size-check` stays green and both `sidebar.rs` and the new module are under 700 lines.
- [x] 4.3 Branch `agent_rows` in `render/sidebar.rs` between the full and compact row on the predicate, without duplicating the click, selection or menu wiring; verify a right-click on a compact row opens the same menu as at full width and clicking one selects that agent.
- [x] 4.4 Make the dashboard row icon-only when compact in `render/sidebar.rs`'s `dashboard_row`, keeping its selected background and click behavior; verify toggling the dashboard still works from a compact sidebar.
- [x] 4.5 Hide the app-name label in the sidebar column's `TitleBar` when compact, keeping the app icon, and make the footer "New agent" button icon-only with its meaning as a tooltip; verify at 120px that the traffic lights, the icon and the button are all visible and the button still opens the new-agent dialog.
- [ ] 4.6 Verify the breakpoint both ways: dragging from 250px to 140px switches all four surfaces to compact, and dragging back to 200px restores every name and detail line.

## 5. Tests and checks

- [x] 5.1 Add a unit test for the compact predicate covering below, at and above the breakpoint, registered in `crates/knot/src/tests/mod.rs`; verify `make test` runs it.
- [ ] 5.2 Walk `specs/agent-list-ui/spec.md` scenario by scenario against the running app - widen, narrow, cursor, both bounds, header alignment, reopen, second window, open-window independence, concurrent settings edit, the breakpoint both ways, tooltip, state dot, stopped dimming, context menu, selection.
- [x] 5.3 Run `make` and `make size-check` and verify the whole workspace is clean.
