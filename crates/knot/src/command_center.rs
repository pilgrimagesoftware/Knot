use super::*;
pub(crate) struct CommandCenterWindow {
    store:          Arc<Mutex<knot_agents::AgentStore>>,
    settings:       knot_core::Settings,
    dashboard_sort: dashboard::DashboardSort,
}

impl CommandCenterWindow {
    pub(crate) fn open(store: Arc<Mutex<knot_agents::AgentStore>>,
                       settings: knot_core::Settings, cx: &mut App) {
        let options = command_center_window_options(cx);
        if let Err(error) =
            cx.open_window(options, move |window, cx| {
                  window.set_window_title(&knot_core::l10n::t("dashboard.command_center"));
                  let view =
                      cx.new(|_| CommandCenterWindow { store,
                                                       settings,
                                                       dashboard_sort:
                                                           dashboard::DashboardSort::default() });
                  cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
              })
        {
            eprintln!("failed to open command center window: {error}");
        }
    }

    /// Folder + insert-after prefill for a workspace's "Add Agent" tile,
    /// matching the Swift reference's `addAgent(to:)`.
    fn add_agent_prefill(&self, workspace_id: Uuid) -> (Option<String>, Option<Uuid>) {
        self.store
            .lock()
            .ok()
            .and_then(|store| {
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
            })
            .unwrap_or((None, None))
    }
}

impl Render for CommandCenterWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dashboard_workspaces = {
            let store = self.store.lock().unwrap();
            store.workspaces()
                 .iter()
                 .map(|workspace| {
                     let dash_agents = workspace.agent_ids
                                                .iter()
                                                .filter_map(|id| store.agent(*id))
                                                .filter(|agent| !agent.is_companion)
                                                .map(|agent| {
                                                    let folder_name =
                                          PathBuf::from(&agent.folder).file_name()
                                                                      .map(|name| {
                                                                          name.to_string_lossy()
                                                                              .into_owned()
                                                                      })
                                                                      .unwrap_or_else(|| {
                                                                          agent.folder.clone()
                                                                      });
                                                    let git_stats =
                                          Repository::open(&agent.folder).diff_stats().ok();
                                                    dashboard::DashboardAgent { id: agent.id,
                                                                  avatar: agent.avatar
                                                                               .graphemes(true)
                                                                               .next()
                                                                               .unwrap_or("🤖")
                                                                               .to_string(),
                                                                  name: agent.name.clone(),
                                                                  folder_name,
                                                                  state: agent.state,
                                                                  is_shell: agent.is_shell(),
                                                                  header_title:
                                                                      agent.header_title()
                                                                           .to_string(),
                                                                  git_stats }
                                                })
                                                .collect::<Vec<_>>();
                     dashboard::DashboardWorkspace { id:        workspace.id,
                                                     name:      workspace.name.clone(),
                                                     color_hex: workspace.color_hex.clone(),
                                                     agents:    self.dashboard_sort
                                                                    .sorted(dash_agents), }
                 })
                 .collect::<Vec<_>>()
        };

        let weak = cx.entity().downgrade();

        let sections = dashboard_workspaces.into_iter().map(|workspace| {
            let on_agent_tap = {
                let weak = weak.clone();
                move |id: Uuid, _window: &mut Window, app: &mut gpui_kit::App| {
                    let Some(entity) = weak.upgrade() else {
                        return;
                    };
                    entity.update(app, |view, cx| {
                        let Some(workspace_id) = view.store.lock().ok().and_then(|store| {
                            store
                                .workspaces()
                                .iter()
                                .find(|workspace| workspace.agent_ids.contains(&id))
                                .map(|workspace| workspace.id)
                        }) else {
                            return;
                        };
                        WorkspaceWindow::open_with_selection(
                            Arc::clone(&view.store),
                            view.settings.clone(),
                            workspace_id,
                            Some(id),
                            cx,
                        );
                    });
                }
            };
            let on_workspace_nav = {
                let weak = weak.clone();
                move |workspace_id: Uuid, _window: &mut Window, app: &mut gpui_kit::App| {
                    let Some(entity) = weak.upgrade() else {
                        return;
                    };
                    entity.update(app, |view, cx| {
                        WorkspaceWindow::open(
                            Arc::clone(&view.store),
                            view.settings.clone(),
                            workspace_id,
                            cx,
                        );
                    });
                }
            };
            let on_add_agent = {
                let weak = weak.clone();
                move |workspace_id: Uuid, _window: &mut Window, app: &mut gpui_kit::App| {
                    let Some(entity) = weak.upgrade() else {
                        return;
                    };
                    entity.update(app, |view, cx| {
                        let (folder, insert_after) = view.add_agent_prefill(workspace_id);
                        let store = Arc::clone(&view.store);
                        let settings = view.settings.clone();
                        let on_created = move |id: Uuid, _window: &mut Window, cx: &mut App| {
                            WorkspaceWindow::open_with_selection(
                                Arc::clone(&store),
                                settings.clone(),
                                workspace_id,
                                Some(id),
                                cx,
                            );
                        };
                        open_agent_editor(
                            Arc::clone(&view.store),
                            view.settings.clone(),
                            AgentEditorRequest {
                                workspace_id,
                                prefill_folder: folder,
                                insert_after,
                                edit_target: None,
                            },
                            on_created,
                            cx,
                        );
                    });
                }
            };

            dashboard::workspace_section(
                workspace,
                true,
                on_agent_tap,
                on_workspace_nav,
                on_add_agent,
            )
        });

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                TitleBar::new()
                    .border_color(gpui_kit::transparent_black())
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(app_titlebar_icon())
                            .child(knot_core::l10n::t("dashboard.command_center")),
                    ),
            )
            .child(
                h_flex()
                    .px_5()
                    .items_center()
                    .justify_end()
                    .child(dashboard::sort_picker(self.dashboard_sort, {
                        let weak = weak.clone();
                        move |sort, _window, app| {
                            let Some(entity) = weak.upgrade() else {
                                return;
                            };
                            entity.update(app, |view, cx| {
                                view.dashboard_sort = sort;
                                cx.notify();
                            });
                        }
                    })),
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_6()
                    .p_6()
                    .overflow_hidden()
                    .children(sections),
            )
            .children(crate::app_support::root_overlays(window, cx))
    }
}
