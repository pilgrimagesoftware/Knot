//! The agent list's background context menu - the one you get by
//! right-clicking the sidebar itself rather than a row.
//!
//! Shape and enablement come from
//! [`crate::app_state::sidebar_background_menu_entries`], which is pure and
//! tested; this is the wiring that reads the facts it needs out of the
//! store and runs the action the user picked.

use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Window;
use gpui_kit::component::menu::PopupMenu;
use gpui_kit::component::menu::PopupMenuItem;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::app_state::AgentListBackgroundEntry;
use crate::app_state::SidebarMenuFacts;
use crate::app_state::sidebar_background_menu_entries;
use crate::broadcast_sheet::open_broadcast_sheet;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::menus::confirm_then;

/// What the sidebar's background context menu acts on. The workspace, not
/// any one agent: every item here is scoped to the workspace the sidebar is
/// showing, as the reference's `currentWorkspaceAgents` is.
#[derive(Clone)]
pub(crate) struct SidebarMenuTargets {
    pub(crate) store:         Arc<Mutex<knot_agents::AgentStore>>,
    pub(crate) window_entity: Entity<WorkspaceWindow>,
    pub(crate) workspace_id:  Uuid,
}

/// Every agent id in `workspace_id`, copied out of the store.
///
/// A snapshot rather than a live borrow, and the reason each bulk action
/// takes one before it starts: `AgentStore::remove` mutates the workspace's
/// agent list - and takes each removed agent's companions with it - so a
/// loop reading that list as it shrinks skips agents, which is how half a
/// workspace survives a Close All. The Swift reference copies first for the
/// same reason (`SidebarView.swift`'s `agentsToClose`).
pub(crate) fn workspace_agent_ids(store: &knot_agents::AgentStore, workspace_id: Uuid)
                                  -> Vec<Uuid> {
    store.workspaces()
         .iter()
         .find(|workspace| workspace.id == workspace_id)
         .map(|workspace| workspace.agent_ids.clone())
         .unwrap_or_default()
}

/// Reads the workspace's agent counts for the sidebar's background menu.
///
/// Pure over the store, like [`agent_menu_facts`], so the enablement rules
/// are testable without a window, and read when the menu opens rather than
/// when the sidebar renders, so the item set reflects the store's current
/// state. "Running" is `activated` - the same signal the row menu's
/// Deactivate is gated on.
pub(crate) fn sidebar_menu_facts(store: &knot_agents::AgentStore, workspace_id: Uuid)
                                 -> SidebarMenuFacts {
    let Some(workspace) = store.workspaces()
                               .iter()
                               .find(|workspace| workspace.id == workspace_id)
    else {
        return SidebarMenuFacts::default();
    };
    let agents = workspace.agent_ids
                          .iter()
                          .filter_map(|id| store.agent(*id))
                          .collect::<Vec<_>>();
    SidebarMenuFacts { agent_count:   agents.len(),
                       running_count: agents.iter().filter(|agent| agent.activated).count(),
                       bench_count:   0, }
}

/// Builds the sidebar's background context menu: every entry
/// [`sidebar_background_menu_entries`] returns, with the ones that do not
/// apply rendered disabled rather than omitted.
///
/// The bench is read from the live settings surface as the menu opens, so an
/// entry saved from another window or from Settings is listed.
pub(crate) fn sidebar_background_context_menu(targets: &SidebarMenuTargets, menu: PopupMenu,
                                              window: &mut Window, cx: &mut Context<PopupMenu>)
                                              -> PopupMenu {
    let bench = crate::settings_global::read(cx).bench_agents.clone();
    let facts = SidebarMenuFacts { bench_count: bench.len(),
                                   ..sidebar_menu_facts(&targets.store.lock(),
                                                        targets.workspace_id) };
    let mut menu = menu;
    for item in sidebar_background_menu_entries(facts) {
        menu = match item.entry.label() {
            None => menu.separator(),
            // A submenu with nothing in it would open empty, so an empty
            // bench shows the item disabled instead, keeping its position.
            Some(label) if item.entry == AgentListBackgroundEntry::NewFromBench && item.enabled => {
                let targets = targets.clone();
                let bench = bench.clone();
                menu.submenu(label, window, cx, move |submenu, _, _| {
                        bench_submenu(submenu, &bench, &targets)
                    })
            }
            Some(label) => {
                let targets = targets.clone();
                let entry = item.entry;
                menu.item(PopupMenuItem::new(label).disabled(!item.enabled)
                                                   .on_click(move |_, window, app| {
                                                       run_sidebar_menu_action(entry, &targets,
                                                                               window, app);
                                                   }))
            }
        };
    }
    menu
}

/// The New from Bench submenu: each entry by avatar and name, in bench
/// order, deploying into the sidebar's workspace.
fn bench_submenu(mut submenu: PopupMenu, bench: &[knot_core::BenchAgent],
                 targets: &SidebarMenuTargets)
                 -> PopupMenu {
    for entry in bench {
        let label = format!("{} {}", entry.avatar, entry.name);
        let entry = entry.clone();
        let targets = targets.clone();
        submenu = submenu.item(PopupMenuItem::new(label).on_click(move |_, _, app| {
                                   targets.window_entity.update(app, |view, cx| {
                                                            view.deploy_bench_entry(&entry, cx);
                                                            cx.notify();
                                                        });
                               }));
    }
    submenu
}

/// Runs one item of the sidebar's background menu.
///
/// The two confirming items open through `window.defer` for the same reason
/// the row menu's do: a `PopupMenu` dismisses itself after running a handler
/// and takes an inline dialog down with it.
fn run_sidebar_menu_action(entry: AgentListBackgroundEntry, targets: &SidebarMenuTargets,
                           window: &mut Window, app: &mut App) {
    match entry {
        AgentListBackgroundEntry::NewAgent => {
            targets.window_entity.update(app, |view, cx| {
                                     view.open_new_agent_dialog(cx);
                                 });
        }
        AgentListBackgroundEntry::RestartAll => {
            let targets = targets.clone();
            let count = workspace_agent_count(&targets);
            // Decided when the prompt opens, as for the row's Restart Agent:
            // the prompt says whether conversations are kept.
            let keep = crate::settings_global::read(app).restore_conversation_on_launch;
            let body = if keep {
                "menu.sidebar.confirm.restart_all_keep_body"
            }
            else {
                "menu.sidebar.confirm.restart_all_body"
            };
            let description = knot_core::l10n::t_with(body, &[("agents", &agent_count(count))]);
            confirm_then(window,
                         app,
                         knot_core::l10n::t("menu.sidebar.confirm.restart_all_title"),
                         description,
                         move |app| {
                             targets.window_entity.update(app, |view, cx| {
                                                      view.restart_all_agents(keep, cx);
                                                      cx.notify();
                                                  });
                         });
        }
        AgentListBackgroundEntry::CloseAll => {
            let targets = targets.clone();
            let count = workspace_agent_count(&targets);
            let description = knot_core::l10n::t_with("menu.sidebar.confirm.close_all_body",
                                                      &[("agents", &agent_count(count))]);
            confirm_then(window,
                         app,
                         knot_core::l10n::t("menu.sidebar.confirm.close_all_title"),
                         description,
                         move |app| {
                             targets.window_entity.update(app, |view, cx| {
                                                      view.close_all_agents(cx);
                                                      cx.notify();
                                                  });
                         });
        }
        AgentListBackgroundEntry::DeactivateAll => {
            // No confirmation, matching the row menu's Deactivate: every
            // agent it stops comes back by being selected, which is the
            // test Restart All and Close All fail.
            targets.window_entity.update(app, |view, cx| {
                                     view.deactivate_all_agents();
                                     cx.notify();
                                 });
        }
        AgentListBackgroundEntry::Broadcast => {
            let targets = targets.clone();
            window.defer(app, move |_window, app| {
                      let window_entity = targets.window_entity.clone();
                      open_broadcast_sheet(move |text, _window, app| {
                                               window_entity.update(app, |view, cx| {
                                                                view.broadcast_to_agents(&text);
                                                                cx.notify();
                                                            });
                                           },
                                           app);
                  });
        }
        // Opens its submenu rather than running anything.
        AgentListBackgroundEntry::NewFromBench | AgentListBackgroundEntry::Separator => {}
    }
}

/// How many agents the workspace holds right now, for a confirmation that
/// names the count.
fn workspace_agent_count(targets: &SidebarMenuTargets) -> usize {
    sidebar_menu_facts(&targets.store.lock(), targets.workspace_id).agent_count
}

/// "agent" or "agents", so a confirmation naming one agent reads as English.
fn agent_count(count: usize) -> String {
    knot_core::l10n::pluralize(count as u64, "count.agent", "count.agents")
}
