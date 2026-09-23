//! The dashboard: the workspace overview that replaces the agent pane
//! when the dashboard row is selected.
//!
//! The cards themselves are [`crate::dashboard`]'s; this assembles the data
//! they need and wires their callbacks back to the window.

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::div;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::agent_editor::AgentEditorRequest;
use crate::agent_editor::AgentPrefill;
use crate::agent_editor::open_agent_editor;
use crate::consts;
use crate::dashboard;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// The dashboard pane, or `None` when the window is not showing it.
    ///
    /// Diff stats come from the cache rather than from `git` directly - a
    /// card per agent times a subprocess per render is what
    /// [`WorkspaceWindow::refresh_dashboard_diff_stats`] exists to avoid.
    pub(super) fn dashboard_content(&mut self, is_dashboard: bool, cx: &mut Context<Self>)
                                    -> Option<gpui_kit::AnyElement> {
        let dashboard_diff_stats = if is_dashboard {
            self.diff_stats_snapshot()
        }
        else {
            BTreeMap::new()
        };

        let dashboard_workspace =
            is_dashboard.then(|| {
                            let store = self.store.lock();
                            let workspace =
                                store.workspaces()
                                     .iter()
                                     .find(|workspace| workspace.id == self.workspace_id);
                            let (name, color_hex, dash_agents) = match workspace {
                                Some(workspace) => {
                                    let dash_agents = workspace.agent_ids
                                                               .iter()
                                                               .filter_map(|id| store.agent(*id))
                                                               .filter(|agent| !agent.is_companion)
                                                               .map(|agent| {
                                                                   let folder_name =
                                                                       agent.folder_name();
                                                                   let git_stats =
                                dashboard_diff_stats.get(&agent.id).copied().flatten();
                                                                   dashboard::DashboardAgent {
                                id: agent.id,
                                avatar: agent
                                    .avatar
                                    .graphemes(true)
                                    .next()
                                    .unwrap_or(consts::DEFAULT_AGENT_AVATAR)
                                    .to_string(),
                                name: agent.name.clone(),
                                folder_name,
                                state: agent.state,
                                is_shell: agent.is_shell(),
                                header_title: agent.header_title().to_string(),
                                git_stats,
                                is_running: agent.activated,
                            }
                                                               })
                                                               .collect::<Vec<_>>();
                                    (workspace.name.clone(),
                                     workspace.color_hex.clone(),
                                     dash_agents)
                                }
                                None => (String::new(), "#1B4FB2".to_string(), Vec::new()),
                            };
                            dashboard::DashboardWorkspace { id: self.workspace_id,
                                                            name,
                                                            color_hex,
                                                            agents: self.dashboard_sort
                                                                        .sorted(dash_agents) }
                        });

        let weak = cx.entity().downgrade();

        dashboard_workspace.map(|dashboard_workspace| {
            let on_agent_tap = {
                let weak = weak.clone();
                move |id: Uuid, _window: &mut Window, app: &mut gpui_kit::App| {
                    if let Some(entity) = weak.upgrade() {
                        entity.update(app, |view, cx| {
                                  view.select_agent(id);
                                  view.view_mode = WorkspaceViewMode::Terminal;
                                  view.ensure_session(id, cx);
                                  view.ensure_panel_session(id, cx);
                                  cx.notify();
                              });
                    }
                }
            };
            let on_workspace_nav = |_id: Uuid, _window: &mut Window, _app: &mut gpui_kit::App| {};
            let on_add_agent = {
                let weak = weak.clone();
                let store = Arc::clone(&self.store);
                move |workspace_id: Uuid, _window: &mut Window, app: &mut gpui_kit::App| {
                    let (folder, insert_after) =
                        {
                            let store = store.lock();
                            store.workspaces()
                                 .iter()
                                 .find(|workspace| workspace.id == workspace_id)
                                 .map(|workspace| {
                                     let folder = workspace.agent_ids
                                                           .iter()
                                                           .filter_map(|id| store.agent(*id))
                                                           .next()
                                                           .map(|agent| agent.folder.clone());
                                     (folder, workspace.agent_ids.last().copied())
                                 })
                        }.unwrap_or((None, None));
                    if let Some(entity) = weak.upgrade() {
                        entity.update(app, |view, cx| {
                                  let on_created =
                                      WorkspaceWindow::select_and_focus_created_agent(cx);
                                  open_agent_editor(
                                Arc::clone(&view.store),
                                AgentEditorRequest {
                                    workspace_id,
                                    prefill: AgentPrefill {
                                        folder,
                                        ..Default::default()
                                    },
                                    insert_after,
                                    edit_target: None,
                                },
                                on_created,
                                cx,
                            );
                              });
                    }
                }
            };

            v_flex().size_full()
                    .child(div().id("workspace-dashboard-grid")
                                .size_full()
                                .p_6()
                                .overflow_y_scroll()
                                .child(dashboard::workspace_section(
                        dashboard_workspace,
                        false,
                        cx.theme().muted_foreground,
                        dashboard::WorkspaceSectionCallbacks { on_agent_tap,
                                                               on_workspace_nav,
                                                               on_add_agent },
                    )))
                    .into_any_element()
        })
    }
}
