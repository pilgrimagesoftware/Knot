//! The content column: everything to the right of the sidebar - the
//! selected agent's header strip and the pane below it (terminal, panel,
//! markdown viewer, or the dashboard).

use super::super::*;

impl WorkspaceWindow {
    /// The column that fills the window beside the sidebar.
    ///
    /// `min_w_0` throughout: a wide panel message (a markdown table, a long
    /// command line) must wrap inside this column rather than stretch it
    /// past the window and push the composer's Send button off screen - see
    /// `.claude/rules/knot-ui-conventions.md`, "Flex overflow".
    pub(super) fn content_column(&mut self, is_dashboard: bool,
                                 dashboard_content: Option<gpui_kit::AnyElement>,
                                 title_bar_left: gpui_kit::AnyElement,
                                 title_bar_right: gpui_kit::AnyElement, window: &mut Window,
                                 cx: &mut Context<Self>)
                                 -> gpui_kit::AnyElement {
        // `min_w_0` so a wide panel message (a markdown table, a
        // long command line) wraps inside this column instead of
        // stretching it past the window and pushing the prompt
        // input's Send button off screen - see
        // `knot-ui-conventions.md`'s "Flex overflow" rule.
        v_flex().flex_1()
                .min_w_0()
                .h_full()
                .children((!is_dashboard).then(|| {
                                             // `min_w_0` on the row and a
                                             // non-shrinking right
                                             // side: at a narrow window the
                                             // agent's status line
                                             // used to push the whole header
                                             // wider than the
                                             // pane, clipping the title on one
                                             // edge and running
                                             // the diff stat off the other.
                                             h_flex().w_full()
                                                     .min_w_0()
                                                     .flex_shrink_0()
                                                     .h(px(64.))
                                                     .items_center()
                                                     .justify_between()
                                                     .gap_3()
                                                     .px_5()
                                                     .bg(cx.theme().background)
                                                     .child(title_bar_left)
                                                     .child(h_flex().flex_shrink_0()
                                                                    .items_center()
                                                                    .gap_2()
                                                                    .child(title_bar_right))
                                         }))
                .child(dashboard_content.unwrap_or_else(|| {
                                            // `flex_1().min_h_0()`, not
                                            // `size_full()`: this box
                                            // is a sibling of the 64px title
                                            // bar above it, so a
                                            // full height makes it overflow its
                                            // container by
                                            // exactly that much and pushes the
                                            // input area's
                                            // control row and Send button below
                                            // the window edge.
                                            v_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(
                        self.selected_agent
                                    .and_then(|id| {
                                        let (is_panel_mode, markdown_file, stopped) = {
                                            let store = self.store.lock().unwrap();
                                            let agent = store.agent(id);
                                            (agent.map(|agent| agent.view_mode)
                                             == Some(knot_core::ViewMode::Panel),
                                             agent.and_then(|agent| {
                                                      agent.markdown_file.clone()
                                                  }),
                                             agent.filter(|agent| !agent.activated)
                                                  .map(|agent| agent.name.clone()))
                                        };
                                        // Ahead of both session
                                        // panes: an open markdown
                                        // file takes the content
                                        // area, whichever mode the
                                        // agent otherwise runs in.
                                        if let Some(file) = markdown_file {
                                            return Some(self.render_markdown_pane(id,
                                                                                  &file,
                                                                                  cx));
                                        }
                                        // Ahead of both session
                                        // panes, which would
                                        // otherwise render empty:
                                        // there is no session, and
                                        // asking for one is what
                                        // the activation gate
                                        // refuses.
                                        if let Some(name) = stopped {
                                            return Some(self.render_stopped_pane(name,
                                                                                 cx));
                                        }
                                        if is_panel_mode {
                                            return Some(self.render_panel_pane(id,
                                                                               window,
                                                                               cx));
                                        }
                                        let grid =
                                            self.sessions.get(&id)?.lock().ok()?.grid()?;
                                        Some(
                                            div()
                                                .id("terminal-pane")
                                                .size_full()
                                                .track_focus(&self.terminal_focus)
                                                .on_mouse_down(
                                                    gpui_kit::MouseButton::Left,
                                                    cx.listener(
                                                        move |view,
                                                              event: &gpui_kit::MouseDownEvent,
                                                              window,
                                                              cx| {
                                                            view.terminal_focus
                                                                .clone()
                                                                .focus(window, cx);
                                                            view.dispatch_mouse_button(
                                                                id,
                                                                event.position,
                                                                knot_terminal::MouseButton::Left,
                                                                true,
                                                                cx,
                                                            );
                                                        },
                                                    ),
                                                )
                                                .on_mouse_up(
                                                    gpui_kit::MouseButton::Left,
                                                    cx.listener(
                                                        move |view,
                                                              event: &gpui_kit::MouseUpEvent,
                                                              _window,
                                                              cx| {
                                                            view.dispatch_mouse_button(
                                                                id,
                                                                event.position,
                                                                knot_terminal::MouseButton::Left,
                                                                false,
                                                                cx,
                                                            );
                                                        },
                                                    ),
                                                )
                                                .on_mouse_move(cx.listener(
                                                    move |view,
                                                          event: &gpui_kit::MouseMoveEvent,
                                                          _window,
                                                          cx| {
                                                        if event.dragging() {
                                                            view.dispatch_mouse_drag(
                                                                id,
                                                                event.position,
                                                                cx,
                                                            );
                                                        }
                                                    },
                                                ))
                                                .on_scroll_wheel(cx.listener(
                                                    move |view,
                                                          event: &gpui_kit::ScrollWheelEvent,
                                                          _window,
                                                          cx| {
                                                        let (_, cell_height) =
                                                            terminal_cell_size(
                                                                cx,
                                                                terminal_font_family(
                                                                    &view.settings,
                                                                    cx,
                                                                ),
                                                                px(view
                                                                    .settings
                                                                    .terminal_font_size
                                                                    as f32),
                                                            );
                                                        let lines = match event.delta {
                                                            gpui_kit::ScrollDelta::Lines(
                                                                point,
                                                            ) => point.y,
                                                            gpui_kit::ScrollDelta::Pixels(
                                                                point,
                                                            ) => {
                                                                f32::from(point.y)
                                                                    / cell_height
                                                            }
                                                        };
                                                        view.dispatch_scroll(
                                                            id,
                                                            event.position,
                                                            lines,
                                                            cx,
                                                        );
                                                    },
                                                ))
                                                .on_key_down(cx.listener(
                                                    move |view, event, _window, cx| {
                                                        view.dispatch_key(id, event, cx);
                                                    },
                                                ))
                                                .child(terminal_view::render_grid(
                                                    &grid.lock().unwrap(),
                                                    terminal_font_family(
                                                        &self.settings,
                                                        cx,
                                                    ),
                                                    px(self.settings.terminal_font_size
                                                        as f32),
                                                ))
                                                .into_any_element(),
                                        )
                                    })
                                    .unwrap_or_else(|| {
                                        div()
                                            .size_full()
                                            .p_6()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .bg(cx.theme().muted)
                                            .child(
                                                div()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(if self.selected_agent.is_some()
                                                    {
                                                        "Starting terminal…"
                                                    } else {
                                                        "Choose an agent from the sidebar"
                                                    }),
                                            )
                                            .into_any_element()
                                    }),
                            )
                            .into_any_element()
                                        }))
                .into_any_element()
    }
}
