//! The workspace window's element tree.
//!
//! [`Render::render`] here is a dispatcher: it snapshots the state the
//! frame needs out of the store once, then hands each region to the module
//! that owns it - [`sidebar`] for the agent list, [`dashboard`] for the
//! overview, [`title_bar`] for the header, [`content`] for the pane that
//! fills the rest.
//!
//! Splitting it that way is not cosmetic. Every one of those regions
//! captures the same handful of locals and builds closures over the window
//! entity; keeping them in one function meant a change to any region had
//! to be read against all of them.

use super::*;

mod content;
mod overview;
mod sidebar;
mod title_bar;

impl Render for WorkspaceWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Manrope applies explicitly to header/cell text that isn't the
        // agent's name - the name (a "title") keeps the app-wide default
        // font (Adamina), so it needs no override here.
        let ui_font_name = self.settings.ui_font_name.clone();
        let ui_font_size = px(self.settings.ui_font_size as f32);
        let (_workspace_name, agents) = {
            let store = self.store.lock().unwrap();
            let Some(workspace) = store.workspaces()
                                       .iter()
                                       .find(|workspace| workspace.id == self.workspace_id)
            else {
                return v_flex().size_full()
                               .child(TitleBar::new().border_color(gpui_kit::transparent_black()))
                               .child("Workspace no longer exists.");
            };
            let agents =
                workspace.agent_ids
                         .iter()
                         .filter_map(|id| store.agent(*id))
                         .map(|agent| {
                             let persona_name =
                                 agent.persona_id.and_then(|id| {
                                                     self.settings
                                                         .personas
                                                         .iter()
                                                         .find(|persona| persona.id == id)
                                                         .map(|persona| persona.name.clone())
                                                 });
                             AgentRow { id: agent.id,
                                        avatar: agent.avatar.clone(),
                                        name: agent.name.clone(),
                                        folder: agent.folder.clone(),
                                        state: agent.state,
                                        is_shell: agent.is_shell(),
                                        is_companion: agent.is_companion,
                                        header_title: agent.header_title().to_string(),
                                        persona_name,
                                        agent_type: agent.agent_type.clone(),
                                        is_running: agent.activated }
                         })
                         .collect::<Vec<_>>();
            (workspace.name.clone(), agents)
        };

        let is_dashboard = self.view_mode == WorkspaceViewMode::Dashboard;

        if !is_dashboard && let Some(id) = self.selected_agent {
            self.resize_session_to_pane(id, window, cx);
        }
        // A map lookup and an `Instant` compare per render; the `git`
        // subprocess behind it runs at most every `DIFF_STATS_MAX_AGE`.
        if let Some(id) = self.selected_agent {
            let folder = self.store
                             .lock()
                             .ok()
                             .and_then(|store| store.agent(id).map(|agent| agent.folder.clone()));
            if let Some(folder) = folder {
                self.refresh_diff_stats(id, &folder);
            }
        }
        if is_dashboard {
            self.refresh_dashboard_diff_stats();
        }

        // See `root_focus`: without this the Agents menu's items are never
        // on the dispatch path macOS validates them against. Done here
        // rather than beside the element it focuses, because the agent rows
        // built below borrow `cx` until the tree is assembled.
        if window.focused(cx).is_none() {
            window.focus(&self.root_focus.clone(), cx);
        }

        let agent_rows = self.agent_rows(agents, ui_font_name.clone(), ui_font_size, cx);
        let dashboard_row = self.dashboard_row(is_dashboard, cx);

        let selected_header = self.selected_agent_header();

        let dashboard_content = self.dashboard_content(is_dashboard, cx);

        let title_bar_left = self.title_bar_left(is_dashboard,
                                                 &selected_header,
                                                 &ui_font_name,
                                                 ui_font_size,
                                                 cx);
        let title_bar_right = self.title_bar_right(is_dashboard,
                                                   &selected_header,
                                                   &ui_font_name,
                                                   ui_font_size,
                                                   cx);
        let selected_menu = self.selected_agent_menu(cx);
        let background_targets = SidebarMenuTargets { store:         Arc::clone(&self.store),
                                                      window_entity: cx.entity(),
                                                      workspace_id:  self.workspace_id, };
        h_flex()
            .size_full()
            .map(|el| with_agents_menu_actions(el, selected_menu.as_ref()))
            .track_focus(&self.root_focus)
            .on_action(cx.listener(|view, _: &PanelPermissionAllow, _, cx| {
                view.answer_selected_permission(knot_acp::PermissionDecision::Allow);
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &PanelPermissionDeny, _, cx| {
                view.answer_selected_permission(knot_acp::PermissionDecision::Deny);
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &PanelOpenPermissionSelector, _, cx| {
                if view.selected_agent.is_some_and(|id| {
                    view.store.lock().ok().and_then(|store| {
                        store.agent(id).map(|agent| agent.view_mode)
                    }) == Some(knot_core::ViewMode::Panel)
                }) {
                    view.open_config_selector = Some(PERMISSION_SELECTOR_ID);
                    cx.notify();
                }
            }))
            .child(
                // The sidebar column owns the traffic lights (Swift's own
                // sidebar panel does the same - they sit within its width,
                // not the content pane's). The content header below is a
                // plain sibling row, not part of this TitleBar, so it
                // starts at this column's true right edge with no gutter
                // GPUI reserves inside TitleBar for the traffic lights -
                // that's what kept misaligning it with the divider below.
                v_flex()
                    .w(px(250.))
                    .h_full()
                    .flex_shrink_0()
                    .bg(cx.theme().title_bar)
                    .child(
                        TitleBar::new()
                            .h(px(window_options::WORKSPACE_TITLE_BAR_HEIGHT))
                            .border_color(gpui_kit::transparent_black())
                            .bg(cx.theme().title_bar)
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(app_titlebar_icon())
                                    .child(knot_core::l10n::t("app.name")),
                            ),
                    )
                    .child(
                        div()
                            .id("workspace-agent-list")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(v_flex().min_h_full()
                                           .gap_1()
                                           .p_4()
                                           .child(dashboard_row)
                                           .children(agent_rows)
                                           // The background menu hangs off a
                                           // filler below the rows rather
                                           // than off the scroll container:
                                           // in GPUI every hitbox under the
                                           // pointer counts as hovered, not
                                           // just the innermost, so a
                                           // container-level context menu
                                           // would open on top of the row's
                                           // own - which `agent-list-ui`
                                           // forbids. A sibling that claims
                                           // the leftover space is reached
                                           // only by a right-click that
                                           // missed every row.
                                           .child(div().id("workspace-agent-list-background")
                                                       .flex_1()
                                                       .min_h(px(32.))
                                                       .context_menu(move |menu, _, _| {
                                                           sidebar_background_context_menu(
                                        &background_targets,
                                        menu,
                                    )
                                                       }))),
                    )
                    .children(
                        self.error
                            .as_ref()
                            .map(|error| div().text_sm().px_4().child(error.clone())),
                    )
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .h(px(48.))
                            .w_full()
                            .items_center()
                            .px_4()
                            .gap_2()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .child(
                                Button::new("workspace-new-agent")
                                    .icon(IconName::Plus)
                                    .label("New agent")
                                    .ghost()
                                    .on_click(cx.listener(
                                        |view, _: &ClickEvent, _window, cx| {
                                            view.open_new_agent_dialog(cx);
                                        },
                                    )),
                            )
                    ),
            )
            .child(self.content_column(is_dashboard,
                                       dashboard_content,
                                       title_bar_left,
                                       title_bar_right,
                                       window,
                                       cx))
                    .children(app_support::root_overlays(window, cx))
    }
}
