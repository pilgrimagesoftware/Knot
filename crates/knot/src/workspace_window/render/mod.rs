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

use std::sync::Arc;

use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::TitleBar;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::component::resizable::ResizableState;
use gpui_kit::component::resizable::h_resizable;
use gpui_kit::component::resizable::resizable_panel;
use gpui_kit::div;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::px;

use crate::app_bootstrap::PanelOpenPermissionSelector;
use crate::app_bootstrap::PanelPermissionAllow;
use crate::app_bootstrap::PanelPermissionDeny;
use crate::app_support;
use crate::app_support::app_titlebar_icon;
use crate::window_options;
use crate::workspace_window::SidebarMenuTargets;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::agent_row::AgentRow;
use crate::workspace_window::composer_focus;
use crate::workspace_window::panel::input::PERMISSION_SELECTOR_ID;
use crate::workspace_window::sidebar_background_context_menu;
use crate::workspace_window::sidebar_is_compact;
use crate::workspace_window::with_agents_menu_actions;

mod content;
mod overview;
pub(super) mod pull_requests_pane;
mod pull_requests_row;
mod sidebar;
mod sidebar_compact;
mod title_bar;

impl WorkspaceWindow {
    /// The rows the sidebar draws, read from the store in one lock, or
    /// `None` when this window's workspace is gone - which is a window that
    /// can only say so.
    fn agent_row_snapshot(&self) -> Option<Vec<AgentRow>> {
        let store = self.store.lock();
        let workspace = store.workspaces()
                             .iter()
                             .find(|workspace| workspace.id == self.workspace_id)?;
        Some(workspace.agent_ids
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
                      .collect())
    }

    /// The work a frame does before it draws: match the terminal to its
    /// pane, ask for diff stats that have aged out, and make sure something
    /// holds focus.
    ///
    /// None of it draws, and none of it runs `git` here - `refresh_diff_stats`
    /// is a map lookup and an `Instant` compare, with the subprocess behind
    /// it running at most every `DIFF_STATS_MAX_AGE`.
    fn prepare_frame(&mut self, is_dashboard: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !is_dashboard && let Some(id) = self.selected_agent {
            self.resize_session_to_pane(id, window, cx);
        }
        if let Some(id) = self.selected_agent {
            let folder = self.store
                             .lock()
                             .agent(id)
                             .map(|agent| agent.folder.clone());
            if let Some(folder) = folder {
                self.refresh_diff_stats(id, &folder);
            }
        }
        if is_dashboard {
            self.refresh_dashboard_diff_stats();
        }

        self.focus_showing_composer(is_dashboard, window, cx);

        // See `root_focus`: without this the Agents menu's items are never
        // on the dispatch path macOS validates them against. Done here
        // rather than beside the element it focuses, because the agent rows
        // built below borrow `cx` until the tree is assembled.
        //
        // After `focus_showing_composer`, which may have taken focus for a
        // composer already - and then this does nothing, correctly: the menu
        // handlers are declared on the root element and the composer is its
        // descendant, the same relationship the terminal pane has.
        if window.focused(cx).is_none() {
            window.focus(&self.root_focus.clone(), cx);
        }
    }

    /// Gives the selected agent's prompt input keyboard focus on the frame
    /// its conversation first appears on, per `acp-panel-ui`'s "Selecting a
    /// Panel-mode agent focuses its prompt input".
    ///
    /// The comparison is against the composer the frame is about to *show*,
    /// not against where focus actually is. That is what keeps focus from
    /// being pulled back: once this has focused an agent's composer, no
    /// later frame showing the same agent compares differently, however many
    /// times the window redraws or wherever the user has since clicked.
    fn focus_showing_composer(&mut self, is_takeover: bool, window: &mut Window,
                              cx: &mut Context<Self>) {
        let selected = self.selected_agent.and_then(|id| {
                                              let store = self.store.lock();
                                              let agent = store.agent(id)?;
                                              Some(composer_focus::SelectedAgentFacts {
                        id,
                        is_panel_mode: agent.view_mode == knot_core::ViewMode::Panel,
                        has_markdown: agent.markdown_file.is_some(),
                        has_diagram: agent.mermaid_source.is_some(),
                        is_activated: agent.activated,
                    })
                                          });
        let showing = composer_focus::showing_composer(is_takeover, selected.as_ref());

        // Stored whether or not focus is taken below, so a frame skipped for
        // an open dialog is not replayed as a transition once it closes -
        // the dialog's own scenario is that focus stays with the dialog, and
        // by then the selection is no longer news.
        let changed = showing != self.focused_composer;
        self.focused_composer = showing;
        if !changed {
            return;
        }
        let Some(id) = showing
        else {
            return;
        };
        // A dialog's focus handle is a descendant of `root_focus` - the
        // dialog layer is a child of the element tracking it - so no
        // containment check can tell a dialog apart from this window's own
        // panes. Asking whether one is open is the only guard that works;
        // `tests/composer_focus.rs` is what establishes that.
        if window.has_active_dialog(cx) {
            return;
        }
        let input = self.panel_prompt_input(id, window, cx);
        input.update(cx, |state, cx| state.focus(window, cx));
    }

    /// The sidebar's own title bar, which owns the traffic lights.
    ///
    /// Compact drops the application name and keeps the icon: at this width
    /// the label has nowhere to go but into the traffic lights.
    fn sidebar_title_bar(compact: bool, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        TitleBar::new().h(px(window_options::WORKSPACE_TITLE_BAR_HEIGHT))
                       .border_color(gpui_kit::transparent_black())
                       .bg(cx.theme().title_bar)
                       .child(h_flex().gap_2()
                                      .items_center()
                                      .child(app_titlebar_icon())
                                      .when(!compact, |row| {
                                          row.child(knot_core::l10n::t("app.name"))
                                      }))
    }

    /// The row under the agent list. Compact keeps the icon and moves the
    /// label into a tooltip, so the control still says what it does.
    fn new_agent_button(&self, compact: bool, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        h_flex().flex_shrink_0()
                .h(px(48.))
                .w_full()
                .items_center()
                .px_4()
                .gap_2()
                .when(compact, |row| row.justify_center())
                .border_t_1()
                .border_color(cx.theme().border)
                .child(Button::new("workspace-new-agent").icon(IconName::Plus)
                                                         .when(!compact, |button| {
                                                             button.label(knot_core::l10n::t("sidebar.new_agent"))
                                                         })
                                                         .when(compact, |button| {
                                                             button.tooltip(knot_core::l10n::t("sidebar.new_agent"))
                                                         })
                                                         .ghost()
                                                         .on_click(cx.listener(|view,
                                                                    _: &ClickEvent,
                                                                    _window,
                                                                    cx| {
                                                             view.open_new_agent_dialog(cx);
                                                         })))
    }

    /// The sidebar column: the window's own title bar, the scrolling agent
    /// list with the dashboard row above it, the error line, and the new
    /// agent button.
    fn sidebar_column(&self, compact: bool, dashboard_row: gpui_kit::AnyElement,
                      pull_requests_row: Option<gpui_kit::AnyElement>,
                      agent_rows: Vec<gpui_kit::AnyElement>,
                      background_targets: SidebarMenuTargets, cx: &mut Context<Self>)
                      -> impl IntoElement + use<> {
        // The sidebar column owns the traffic lights (Swift's own
        // sidebar panel does the same - they sit within its width,
        // not the content pane's). The content header below is a
        // plain sibling row, not part of this TitleBar, so it
        // starts at this column's true right edge with no gutter
        // GPUI reserves inside TitleBar for the traffic lights -
        // that's what kept misaligning it with the divider below.
        v_flex().w_full()
                .h_full()
                .bg(cx.theme().title_bar)
                .child(Self::sidebar_title_bar(compact, cx))
                .child(
                       div().id("workspace-agent-list")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(
            v_flex().min_h_full()
                    .gap_1()
                    .p_4()
                    .child(dashboard_row)
                    .children(pull_requests_row)
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
                    .child(
                div().id("workspace-agent-list-background")
                     .flex_1()
                     .min_h(px(32.))
                     .context_menu(move |menu, _, _| {
                         sidebar_background_context_menu(&background_targets, menu)
                     }),
            ),
        ),
        )
                .children(self.error
                              .as_ref()
                              .map(|error| div().text_sm().px_4().child(error.clone())))
                .child(self.new_agent_button(compact, cx))
    }
}

impl Render for WorkspaceWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The title font (Manrope) applies explicitly to header and cell text
        // that isn't the agent's name - the name keeps the app-wide UI font
        // (Adamina), so it needs no override here.
        let title_font_name = self.settings.title_font_name.clone();
        let title_font_size = px(self.settings.title_font_size as f32);
        let Some(agents) = self.agent_row_snapshot()
        else {
            return v_flex().size_full()
                           .child(TitleBar::new().border_color(gpui_kit::transparent_black()))
                           .child(knot_core::l10n::t("workspace.missing"));
        };
        let is_dashboard = self.view_mode == WorkspaceViewMode::Dashboard;
        let is_pull_requests = self.view_mode == WorkspaceViewMode::PullRequests;
        // Every takeover hides the selected agent's header and pane, so the
        // question the rest of this render asks is "is anything taking the
        // content over", not "is it the dashboard".
        let is_takeover = self.view_mode.is_takeover();

        self.prepare_frame(is_takeover, window, cx);
        // Gated on the view inside: nothing is fetched while it is closed.
        self.refresh_pull_request_states();
        // The one place the compact breakpoint is read. Every surface that
        // changes below it takes this `bool`, so none of them can disagree
        // about where compact begins.
        let sidebar_width = self.sidebar_width(cx);
        let compact = sidebar_is_compact(sidebar_width);

        let agent_rows = self.agent_rows(agents,
                                         title_font_name.clone(),
                                         title_font_size,
                                         compact,
                                         cx);
        let dashboard_row = self.dashboard_row(is_dashboard, compact, cx);
        let pull_requests_row = self.pull_requests_row(compact, cx);

        let selected_header = self.selected_agent_header();

        // One content slot: at most one takeover shows at a time, so the
        // first that claims it wins and `content_column` needs no third arm.
        let takeover_content = self.dashboard_content(is_dashboard, cx)
                                   .or_else(|| self.pull_requests_content(is_pull_requests, cx));

        let title_bar_left = self.title_bar_left(is_takeover,
                                                 &selected_header,
                                                 &title_font_name,
                                                 title_font_size,
                                                 cx);
        let title_bar_right = self.title_bar_right(is_takeover,
                                                   &selected_header,
                                                   &title_font_name,
                                                   title_font_size,
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
                    view.store.lock().agent(id).map(|agent| agent.view_mode) == Some(knot_core::ViewMode::Panel)
                }) {
                    view.open_config_selector = Some(PERMISSION_SELECTOR_ID);
                    cx.notify();
                }
            }))
            .child(
                // The two columns are the two panels of a resizable group,
                // so the boundary between them is a divider the user drags.
                // The group's handle is absolutely positioned and takes no
                // layout width, which is what keeps the alignment the
                // sidebar column's comment below depends on.
                h_resizable("workspace-columns")
                    .with_state(&self.sidebar_resize)
                    .on_resize(cx.listener(|view, state: &Entity<ResizableState>, _window, cx| {
                        let Some(width) = state.read(cx)
                                               .sizes()
                                               .first()
                                               .map(|width| f64::from(f32::from(*width)))
                        else {
                            return;
                        };
                        view.persist_sidebar_width(width);
                    }))
                    .child(resizable_panel()
                        .size(px(sidebar_width as f32))
                        .size_range(px(knot_core::consts::SIDEBAR_WIDTH_MIN as f32)
                                    ..px(knot_core::consts::SIDEBAR_WIDTH_MAX as f32))
                        // The panel grows by default; a sized panel beside a
                        // flexible one has to opt out or it takes the slack
                        // back on the frame after a drag.
                        .flex_none()
                        .child(self.sidebar_column(compact,
                                                   dashboard_row,
                                                   pull_requests_row,
                                                   agent_rows,
                                                   background_targets,
                                                   cx)))
                    .child(resizable_panel().child(self.content_column(is_takeover,
                                                                       takeover_content,
                                                                       title_bar_left,
                                                                       title_bar_right,
                                                                       window,
                                                                       cx))),
            )
                    .children(app_support::root_overlays(window, cx))
    }
}
