use std::collections::BTreeMap;
use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Root;
use gpui_kit::component::TitleBar;
use knot_git::Repository;
use parking_lot::Mutex;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::agent_editor::AgentEditorRequest;
use crate::agent_editor::AgentPrefill;
use crate::agent_editor::open_agent_editor;
use crate::app_support::app_titlebar_icon;
use crate::app_support::observe_system_appearance;
use crate::consts;
use crate::dashboard;
use crate::diff_stats::DiffStatsCache;
use crate::window_options::command_center_window_options;
use crate::workspace_window::WorkspaceWindow;

pub(crate) struct CommandCenterWindow {
    store:          Arc<Mutex<knot_agents::AgentStore>>,
    messages:       Arc<Mutex<knot_messaging::MessageStore>>,
    dashboard_sort: dashboard::DashboardSort,
    /// Diff stats per agent card. This window shows every workspace's
    /// agents, so it is the worst place to run `git` where the card is
    /// drawn - which is what it did.
    diff_stats:     DiffStatsCache,
}

impl CommandCenterWindow {
    pub(crate) fn open(store: Arc<Mutex<knot_agents::AgentStore>>,
                       messages: Arc<Mutex<knot_messaging::MessageStore>>, cx: &mut App) {
        // One Command Center: it shows every workspace, so a second copy shows
        // exactly what the first does (`openspec/specs/window-lifecycle`).
        crate::window_registry::activate_or_open(
                                                 crate::window_registry::WindowKey::CommandCenter,
                                                 cx,
                                                 move |cx| {
                                                     let options =
                                                         command_center_window_options(cx);
                                                     match cx.open_window(options, move |window, cx| {
                  // Every window tracks the OS appearance, so a light/dark flip
                  // re-resolves the system palette and repaints.
                  observe_system_appearance(window);
                  window.set_window_title(&knot_core::l10n::t("dashboard.command_center"));
                  let view =
                      cx.new(|_| CommandCenterWindow { store,
                                                       messages,
                                                       dashboard_sort:
                                                           dashboard::DashboardSort::default(),
                                                       diff_stats: DiffStatsCache::default() });
                  cx.new(|cx| Root::new(view, window, cx))
              })
            {
                Ok(window) => Some(window.into()),
                Err(error) => {
                    eprintln!("failed to open command center window: {error}");
                    None
                },
            }
                                                 },
        );
    }

    /// Asks for a fresh diff stat for every agent with a card, for the ones
    /// whose cached stat has aged out.
    ///
    /// Called from the render path, which it is allowed to be because it
    /// starts no `git` there: the claim is a map lookup and an `Instant`
    /// compare, and the subprocess runs on a background thread. The answer
    /// reaches the next frame through `cx.notify`, so a card that has no
    /// stat yet simply draws without one and gains it a moment later.
    fn refresh_diff_stats(&mut self, cx: &mut Context<Self>) {
        let folders = {
            let store = self.store.lock();
            store.agents()
                 .iter()
                 .filter(|agent| !agent.is_companion)
                 .map(|agent| (agent.id, agent.folder.clone()))
                 .collect::<Vec<_>>()
        };
        for (id, folder) in folders {
            let Some(writer) = self.diff_stats
                                   .claim_refresh(id, crate::diff_stats::MAX_AGE)
            else {
                continue;
            };
            let view = cx.entity();
            cx.spawn(async move |_this, cx| {
                  let stats = cx.background_executor()
                                .spawn(async move { Repository::open(&folder).diff_stats().ok() })
                                .await;
                  writer.record(stats);
                  cx.update(|app| {
                        view.update(app, |_view, cx| {
                                cx.notify();
                            });
                    });
              })
              .detach();
        }
    }

    /// Opens an agent's workspace window with that agent selected.
    fn on_agent_tap(&self, cx: &mut Context<Self>)
                    -> impl Fn(Uuid, &mut Window, &mut gpui_kit::App) + Clone + 'static + use<>
    {
        let weak = cx.entity().downgrade();
        let weak = weak.clone();
        move |id: Uuid, _window: &mut Window, app: &mut gpui_kit::App| {
            let Some(entity) = weak.upgrade()
            else {
                return;
            };
            entity.update(app, |view, cx| {
                      let Some(workspace_id) =
                          view.store
                              .lock()
                              .workspaces()
                              .iter()
                              .find(|workspace| workspace.agent_ids.contains(&id))
                              .map(|workspace| workspace.id)
                      else {
                          return;
                      };
                      WorkspaceWindow::open_with_selection(Arc::clone(&view.store),
                                                           Arc::clone(&view.messages),
                                                           workspace_id,
                                                           Some(id),
                                                           cx);
                  });
        }
    }

    /// Opens a workspace's own window.
    fn on_workspace_nav(
        &self, cx: &mut Context<Self>)
        -> impl Fn(Uuid, &mut Window, &mut gpui_kit::App) + Clone + 'static + use<> {
        let weak = cx.entity().downgrade();
        let weak = weak.clone();
        move |workspace_id: Uuid, _window: &mut Window, app: &mut gpui_kit::App| {
            let Some(entity) = weak.upgrade()
            else {
                return;
            };
            entity.update(app, |view, cx| {
                      WorkspaceWindow::open(Arc::clone(&view.store),
                                            Arc::clone(&view.messages),
                                            workspace_id,
                                            cx);
                  });
        }
    }

    /// Opens the agent editor prefilled for a workspace, and shows the new
    /// agent once it exists.
    fn on_add_agent(&self, cx: &mut Context<Self>)
                    -> impl Fn(Uuid, &mut Window, &mut gpui_kit::App) + Clone + 'static + use<>
    {
        let weak = cx.entity().downgrade();
        let weak = weak.clone();
        move |workspace_id: Uuid, _window: &mut Window, app: &mut gpui_kit::App| {
            let Some(entity) = weak.upgrade()
            else {
                return;
            };
            entity.update(app, |view, cx| {
                let (folder, insert_after) = view.add_agent_prefill(workspace_id);
                let store = Arc::clone(&view.store);
                let messages = Arc::clone(&view.messages);
                let on_created = move |id: Uuid, _window: &mut Window, cx: &mut App| {
                    WorkspaceWindow::open_with_selection(Arc::clone(&store),
                                                         Arc::clone(&messages),
                                                         workspace_id,
                                                         Some(id),
                                                         cx);
                };
                open_agent_editor(Arc::clone(&view.store),
                                  (*crate::settings_global::read(cx)).clone(),
                                  AgentEditorRequest { workspace_id,
                                                       prefill:
                                                           AgentPrefill { folder,
                                                                          ..Default::default() },
                                                       insert_after,
                                                       edit_target: None },
                                  on_created,
                                  cx);
            });
        }
    }

    /// Every workspace's card set, read from the store in one lock.
    ///
    /// Diff stats come from the cache rather than from `git`: this window
    /// shows every workspace's agents, so computing them here would be a
    /// subprocess per card per frame.
    fn dashboard_workspaces(&self, diff_stats: &BTreeMap<Uuid, Option<knot_git::DiffStats>>)
                            -> Vec<dashboard::DashboardWorkspace> {
        let store = self.store.lock();
        store.workspaces()
                 .iter()
                 .map(|workspace| {
                     let dash_agents = workspace.agent_ids
                                                .iter()
                                                .filter_map(|id| store.agent(*id))
                                                .filter(|agent| !agent.is_companion)
                                                .map(|agent| {
                                                    let folder_name = agent.folder_name();
                                                    let git_stats =
                                                        diff_stats.get(&agent.id)
                                                                  .copied()
                                                                  .flatten();
                                                    dashboard::DashboardAgent { id: agent.id,
                                                                  avatar: agent.avatar
                                                                               .graphemes(true)
                                                                               .next()
                                                                               .unwrap_or(consts::DEFAULT_AGENT_AVATAR)
                                                                               .to_string(),
                                                                  name: agent.name.clone(),
                                                                  folder_name,
                                                                  state: agent.state,
                                                                  is_shell: agent.is_shell(),
                                                                  header_title:
                                                                      agent.header_title()
                                                                           .to_string(),
                                                                  git_stats,
                                                                  is_running: agent.activated }
                                                })
                                                .collect::<Vec<_>>();
                     dashboard::DashboardWorkspace { id:        workspace.id,
                                                     name:      workspace.name.clone(),
                                                     color_hex: workspace.color_hex.clone(),
                                                     agents:    self.dashboard_sort
                                                                    .sorted(dash_agents), }
                 })
                 .collect()
    }

    /// Folder + insert-after prefill for a workspace's "Add Agent" tile,
    /// matching the Swift reference's `addAgent(to:)`.
    fn add_agent_prefill(&self, workspace_id: Uuid) -> (Option<String>, Option<Uuid>) {
        {
            let store = self.store.lock();
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
        }.unwrap_or((None, None))
    }
}

impl Render for CommandCenterWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        self.refresh_diff_stats(cx);
        let diff_stats = self.diff_stats.snapshot();
        let dashboard_workspaces = self.dashboard_workspaces(&diff_stats);
        let weak = cx.entity().downgrade();
        // Built once and cloned per section: every card answers the same
        // three questions, and each takes the id it acts on as an argument.
        let on_agent_tap = self.on_agent_tap(cx);
        let on_workspace_nav = self.on_workspace_nav(cx);
        let on_add_agent = self.on_add_agent(cx);

        let sections = dashboard_workspaces.into_iter()
                                           .map(|workspace| {
                                               dashboard::workspace_section(
                    workspace,
                    true,
                    muted,
                    dashboard::WorkspaceSectionCallbacks { on_agent_tap:     on_agent_tap.clone(),
                                                           on_workspace_nav:
                                                               on_workspace_nav.clone(),
                                                           on_add_agent:     on_add_agent.clone(), },
                )
                                           })
                                           .collect::<Vec<_>>();

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
                // The grid is the Command Center's whole purpose and grows
                // with every workspace and agent, so it scrolls rather than
                // being clipped. Its parent does not scroll, per
                // knot-ui-conventions: a scroll region nested in another one
                // steals the outer gesture.
                v_flex()
                    .id("command-center-grid")
                    .flex_1()
                    .min_h_0()
                    .gap_6()
                    .p_6()
                    .overflow_y_scroll()
                    .children(sections),
            )
            .children(crate::app_support::root_overlays(window, cx))
    }
}
