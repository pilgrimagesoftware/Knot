//! One agent row's context menu.
//!
//! Shape and enablement come from
//! [`crate::app_state::agent_context_menu_entries`], which is pure and
//! tested against the Swift reference's ordering; this is the wiring -
//! reading the row's facts out of the store, building the submenus, and
//! running whichever entry the user picked.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Window;
use gpui_kit::component::menu::PopupMenu;
use gpui_kit::component::menu::PopupMenuItem;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::agent_editor::AgentEditorRequest;
use crate::agent_editor::AgentPrefill;
use crate::agent_editor::open_agent_editor;
use crate::agent_menu::markdown_label;
use crate::app_state::AgentMenuEntry;
use crate::app_state::AgentMenuFacts;
use crate::app_state::agent_context_menu_entries;
use crate::open_in;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::menus::confirm_then;

/// Everything the agent-row context menu's handlers need. Grouped so the
/// builder takes one argument instead of seven, and so the row render can
/// clone it once per row rather than capturing each piece separately.
#[derive(Clone)]
pub(crate) struct AgentMenuTargets {
    pub(crate) store:         Arc<Mutex<knot_agents::AgentStore>>,
    pub(crate) settings:      knot_core::Settings,
    pub(crate) window_entity: Entity<WorkspaceWindow>,
    pub(crate) workspace_id:  Uuid,
    pub(crate) id:            Uuid,
    pub(crate) name:          String,
    pub(crate) folder:        String,
}

/// Reads an agent's menu facts, the workspaces it could move to, and its
/// markdown history out of `store`.
///
/// Pure over the store so the rules that decide the item set - detached
/// workspaces are not move targets, an agent's own workspace is not one
/// either - are testable without a window. Called when the menu opens
/// rather than when the row renders, so a menu never offers a workspace
/// that was closed since the last repaint.
pub(crate) fn agent_menu_facts(store: &knot_agents::AgentStore, id: Uuid)
                               -> (AgentMenuFacts, Vec<(Uuid, String)>, Vec<PathBuf>) {
    let Some(agent) = store.agent(id)
    else {
        return (AgentMenuFacts::default(), Vec::new(), Vec::new());
    };
    let own_workspace = store.workspaces()
                             .iter()
                             .find(|workspace| workspace.agent_ids.contains(&id))
                             .map(|workspace| workspace.id);
    // Detached workspaces live in their own windows and are not move
    // targets, matching the reference's `attachedWorkspaces`.
    let move_targets = store.workspaces()
                            .iter()
                            .filter(|workspace| workspace.is_detached != Some(true))
                            .filter(|workspace| Some(workspace.id) != own_workspace)
                            .map(|workspace| (workspace.id, workspace.name.clone()))
                            .collect::<Vec<_>>();
    let history = agent.markdown_history.clone();
    let facts = AgentMenuFacts { is_companion:         agent.is_companion,
                                 is_shell:             agent.is_shell(),
                                 has_move_targets:     own_workspace.is_some()
                                                       && !move_targets.is_empty(),
                                 has_markdown_history: !history.is_empty(),
                                 is_running:           agent.activated, };
    (facts, move_targets, history)
}

/// Opens the agent editor from a menu handler, notifying the workspace
/// window once the dialog is submitted.
fn open_editor_from_menu(targets: &AgentMenuTargets, prefill: AgentPrefill,
                         insert_after: Option<Uuid>, edit_target: Option<Uuid>, app: &mut App) {
    let window_entity = targets.window_entity.clone();
    open_agent_editor(Arc::clone(&targets.store),
                      targets.settings.clone(),
                      AgentEditorRequest { workspace_id: targets.workspace_id,
                                           prefill,
                                           insert_after,
                                           edit_target },
                      move |_id, _window, app| {
                          window_entity.update(app, |view, cx| {
                                           // Freshly loaded, not this window's
                                           // snapshot: the
                                           // sidebar row resolves the agent's
                                           // persona name
                                           // from `view.settings.personas`,
                                           // which was taken
                                           // when the window opened. Assigning
                                           // a persona
                                           // added since then (or via this same
                                           // edit, on an
                                           // agent that had none) would resolve
                                           // to nothing
                                           // and the row would keep showing no
                                           // persona line.
                                           view.settings =
                                               knot_core::Settings::load().unwrap_or_else(|_| {
                                                                              view.settings.clone()
                                                                          });
                                           cx.notify();
                                       });
                      },
                      app);
}

/// Builds the agent row's context menu: the entries
/// `agent_context_menu_entries` decides on, each with its handler.
///
/// Every dialog opens through `window.defer`. A `PopupMenu` dismisses
/// itself immediately after running a handler, and a dialog opened inline
/// goes down with it; opening on the next turn of the loop lets the menu
/// finish closing first.
pub(crate) fn agent_row_context_menu(targets: &AgentMenuTargets, menu: PopupMenu,
                                     window: &mut Window, cx: &mut Context<PopupMenu>)
                                     -> PopupMenu {
    let (facts, move_targets, markdown_history) =
        agent_menu_facts(&targets.store.lock(), targets.id);
    let mut menu = menu;
    for entry in agent_context_menu_entries(facts) {
        menu = match entry {
            AgentMenuEntry::Separator => menu.separator(),
            AgentMenuEntry::MoveToWorkspace => {
                let targets = targets.clone();
                let move_targets = move_targets.clone();
                menu.submenu("Move to Workspace", window, cx, move |mut submenu, _, _| {
                        for (workspace_id, workspace_name) in &move_targets {
                            let targets = targets.clone();
                            let workspace_id = *workspace_id;
                            submenu =
                            submenu.item(PopupMenuItem::new(workspace_name.clone()).on_click(
                                move |_, _window, app| {
                                    move_agent_to_workspace(&targets, workspace_id, app);
                                },
                            ));
                        }
                        submenu
                    })
            }
            AgentMenuEntry::OpenIn => {
                let folder = targets.folder.clone();
                menu.submenu("Open In…", window, cx, move |mut submenu, _, _| {
                        for item in open_in::open_in_entries() {
                            submenu = match item {
                                open_in::OpenInEntry::Separator => submenu.separator(),
                                open_in::OpenInEntry::App(app_entry) => {
                                    let folder = folder.clone();
                                    submenu.item(PopupMenuItem::new(app_entry.label).on_click(
                                    move |_, _window, _app| {
                                        open_in::open_folder(app_entry.id, &folder);
                                    },
                                ))
                                }
                            };
                        }
                        submenu
                    })
            }
            AgentMenuEntry::MarkdownFiles => {
                let targets = targets.clone();
                let history = markdown_history.clone();
                menu.submenu("Markdown Files", window, cx, move |mut submenu, _, _| {
                        for file in &history {
                            let targets = targets.clone();
                            let file = file.clone();
                            // File name, not the full path: the reference
                            // labels these by `lastPathComponent`, and a
                            // full path makes the submenu unreadable.
                            let label = markdown_label(&file);
                            submenu = submenu.item(PopupMenuItem::new(label).on_click({
                                                       move |_, _window, app| {
                                                           show_agent_markdown_file(&targets,
                                                                                    &file, app);
                                                       }
                                                   }));
                        }
                        submenu
                    })
            }
            entry => {
                let Some(label) = entry.label()
                else {
                    continue;
                };
                let targets = targets.clone();
                menu.item(PopupMenuItem::new(label).on_click(move |_, window, app| {
                                                       run_agent_menu_action(entry, &targets,
                                                                             window, app);
                                                   }))
            }
        };
    }
    menu
}

/// Moves `targets`' agent to `workspace_id`.
///
/// Shared by the row's context menu and the menu bar's Agents menu, which
/// is the point: `app-menu` requires an item to do the same thing from
/// either surface, and the way to guarantee that is one body.
pub(super) fn move_agent_to_workspace(targets: &AgentMenuTargets, workspace_id: Uuid,
                                      app: &mut App) {
    {
        let mut store = targets.store.lock();
        store.move_to_workspace(targets.id, workspace_id);
    }
    targets.window_entity.update(app, |view, cx| {
                             // The moved agent may have been this window's
                             // selection, and
                             // it no longer belongs here.
                             if view.selected_agent == Some(targets.id) {
                                 view.selected_agent = None;
                             }
                             view.persist_agents();
                             cx.notify();
                         });
}

/// Shows `file` in `targets`' agent's markdown pane. Shared by both menus,
/// as [`move_agent_to_workspace`] is.
pub(super) fn show_agent_markdown_file(targets: &AgentMenuTargets, file: &Path, app: &mut App) {
    {
        let mut store = targets.store.lock();
        if let Err(error) = store.set_markdown_panel(targets.id, file.to_path_buf(), false) {
            eprintln!("failed to show {}: {error}", file.display());
        }
    }
    targets.window_entity.update(app, |_, cx| cx.notify());
}

/// Runs one plain (non-submenu) menu entry.
pub(super) fn run_agent_menu_action(entry: AgentMenuEntry, targets: &AgentMenuTargets,
                                    window: &mut Window, app: &mut App) {
    match entry {
        AgentMenuEntry::EditAgent => {
            open_editor_from_menu(targets,
                                  AgentPrefill::default(),
                                  None,
                                  Some(targets.id),
                                  app);
        }
        // The configurable companion: same shape as "New Shell Companion"
        // but routed through the editor, so the user names it and picks
        // its folder before it exists.
        AgentMenuEntry::NewCompanion => {
            let prefill = AgentPrefill { folder: Some(targets.folder.clone()),
                                         agent_type: Some("shell".to_string()),
                                         created_by: Some(targets.id),
                                         is_companion: true,
                                         ..Default::default() };
            open_editor_from_menu(targets, prefill, Some(targets.id), None, app);
        }
        AgentMenuEntry::Deactivate => {
            // No confirmation: nothing is lost that selecting the row will
            // not bring back, which is the test Restart and Remove fail.
            targets.window_entity.update(app, |view, cx| {
                                     view.deactivate_agent(targets.id);
                                     cx.notify();
                                 });
        }
        AgentMenuEntry::NewShellCompanion => {
            let created = targets.store
                                 .lock()
                                 .create_shell_companion(targets.id)
                                 .is_ok();
            if created {
                targets.window_entity.update(app, |view, cx| {
                                         view.persist_agents();
                                         cx.notify();
                                     });
            }
        }
        // A fork carries the source's session so it picks the conversation
        // up; a duplicate deliberately does not.
        AgentMenuEntry::ForkAgent => {
            let source = targets.store.lock().agent(targets.id).cloned();
            let Some(source) = source
            else {
                return;
            };
            let prefill = AgentPrefill { name:         Some(format!("{} (fork)", source.name)),
                                         avatar:       Some(source.avatar.clone()),
                                         folder:       Some(source.folder.clone()),
                                         agent_type:   Some(source.agent_type.clone()),
                                         persona_id:   source.persona_id,
                                         created_by:   None,
                                         is_companion: false,
                                         session_id:   source.session_id.clone(), };
            open_editor_from_menu(targets, prefill, Some(targets.id), None, app);
        }
        AgentMenuEntry::DuplicateAgent => {
            let created = {
                let mut store = targets.store.lock();
                let Some(source) = store.agent(targets.id).cloned()
                else {
                    return;
                };
                store.create(source.folder.clone(),
                             knot_agents::CreateOptions { name: Some(format!("{} (copy)",
                                                                             source.name)),
                                                          avatar: Some(source.avatar.clone()),
                                                          agent_type: Some(source.agent_type
                                                                                 .clone()),
                                                          shell_command: source.shell_command
                                                                               .clone(),
                                                          persona_id: source.persona_id,
                                                          insert_after: Some(targets.id),
                                                          ..Default::default() })
            };
            targets.window_entity.update(app, |view, cx| {
                                     view.persist_agents();
                                     view.select_agent(created);
                                     cx.notify();
                                 });
        }
        // Freshly loaded settings, not this window's snapshot: the bench
        // is edited from the settings window too, and persisting a stale
        // copy would drop whatever was added there since.
        AgentMenuEntry::SaveToBench => {
            let source = targets.store.lock().agent(targets.id).cloned();
            let Some(source) = source
            else {
                return;
            };
            let mut settings =
                knot_core::Settings::load().unwrap_or_else(|_| targets.settings.clone());
            let mut entry = knot_core::BenchAgent::new(Uuid::new_v4(),
                                                       source.name.clone(),
                                                       Some(source.avatar.clone()),
                                                       source.folder.clone());
            entry.agent_type = source.agent_type.clone();
            entry.shell_command = source.shell_command.clone();
            entry.persona_id = source.persona_id;
            if let Err(error) = settings.add_bench_agent(entry) {
                eprintln!("failed to save the agent to the bench: {error}");
            }
        }
        AgentMenuEntry::RegisterAgent => {
            targets.window_entity.update(app, |view, cx| {
                                     view.send_registration_prompt(targets.id);
                                     cx.notify();
                                 });
        }
        AgentMenuEntry::RestartAgent => {
            let targets = targets.clone();
            let description = format!("Restart \"{}\"? Its session will be cleared.", targets.name);
            confirm_then(window, app, "Restart Agent", description, move |app| {
                {
                    let mut store = targets.store.lock();
                    if let Err(error) = store.restart(targets.id) {
                        eprintln!("failed to restart agent {}: {error}", targets.id);
                    }
                }
                targets.window_entity.update(app, |view, cx| {
                                         view.remove_session(targets.id);
                                         view.panel_states.remove(&targets.id);
                                         // `restart` clears the persisted
                                         // session ids; write
                                         // them out so a relaunch doesn't
                                         // resume the session
                                         // just dropped.
                                         view.persist_agents();
                                         cx.notify();
                                     });
            });
        }
        AgentMenuEntry::RemoveAgent => {
            let targets = targets.clone();
            let description = format!("Remove \"{}\"? This closes its session and cannot be \
                                       undone.",
                                      targets.name);
            confirm_then(window, app, "Remove Agent", description, move |app| {
                targets.window_entity.update(app, |view, cx| {
                                         view.remove_agent(targets.id);
                                         cx.notify();
                                     });
            });
        }
        // Handled by the builder, which needs `Window`/`Context` to make
        // a submenu, or carries no action at all.
        AgentMenuEntry::Separator
        | AgentMenuEntry::MoveToWorkspace
        | AgentMenuEntry::OpenIn
        | AgentMenuEntry::MarkdownFiles => {}
    }
}
