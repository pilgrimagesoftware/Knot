//! The macOS menu bar's Agents menu.
//!
//! Unlike the two context menus, this one is app-global: macOS owns it, so
//! a workspace window only *claims* it while it is the active window, and
//! has to rebuild it whenever the facts behind an item change. The handlers
//! are registered on the window's root element rather than on the menu,
//! because macOS validates each item against the dispatch path to the
//! focused node.

use std::sync::Arc;

use gpui_kit::Context;
use gpui_kit::InteractiveElement;

use super::agent_row::{move_agent_to_workspace, run_agent_menu_action, show_agent_markdown_file};
use crate::agent_menu::AgentMenuDeactivate;
use crate::agent_menu::AgentMenuDuplicateAgent;
use crate::agent_menu::AgentMenuEditAgent;
use crate::agent_menu::AgentMenuForkAgent;
use crate::agent_menu::AgentMenuMarkdownFiles;
use crate::agent_menu::AgentMenuMoveToWorkspace;
use crate::agent_menu::AgentMenuMoveToWorkspaceTarget;
use crate::agent_menu::AgentMenuNewCompanion;
use crate::agent_menu::AgentMenuNewShellCompanion;
use crate::agent_menu::AgentMenuOpenIn;
use crate::agent_menu::AgentMenuOpenInApp;
use crate::agent_menu::AgentMenuRegisterAgent;
use crate::agent_menu::AgentMenuRemoveAgent;
use crate::agent_menu::AgentMenuRestartAgent;
use crate::agent_menu::AgentMenuRestartWithNewConversation;
use crate::agent_menu::AgentMenuSaveToBench;
use crate::agent_menu::AgentMenuShowMarkdownFile;
use crate::agent_menu::AgentMenuSnapshot;
use crate::agent_menu::AgentsMenuState;
use crate::app_bootstrap;
use crate::app_state::AgentMenuEntry;
use crate::app_state::agent_context_menu_entries;
use crate::app_support::shorten_path;
use crate::open_in;
use crate::workspace_window::AgentMenuTargets;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::agent_menu_facts;
use crate::workspace_window::creation::SelectedAgentHeader;

/// Registers the Agents menu's action handlers on `el`, one per item the
/// selected agent can actually use.
///
/// Every item is in the menu; only the applicable ones get a handler, and
/// macOS greys the rest by asking gpui whether each action is available.
/// With nothing selected no handler is registered at all, so the whole menu
/// is disabled (`app-menu`).
macro_rules! agents_menu_handlers {
    ($el:expr, $selected:expr, [ $( $entry:ident => $action:ty ),* $(,)? ]) => {{
        let mut el = $el;
        if let Some(selected) = $selected {
            $(
                if selected.snapshot.entries.contains(&AgentMenuEntry::$entry) {
                    let targets = selected.targets.clone();
                    el = el.on_action(move |_: &$action, window, app| {
                               run_agent_menu_action(AgentMenuEntry::$entry, &targets, window, app);
                           });
                }
            )*
        }
        el
    }};
}

impl WorkspaceWindow {
    /// Rebuilds the menu bar when the Agents menu's submenus would now
    /// list something different.
    ///
    /// A `Menu` is a static snapshot: Move to Workspace and Markdown Files
    /// cannot re-read the store when they open, so a workspace created or
    /// a markdown file shown since the bar was built would be missing from
    /// them (`app-menu`). Everything else in the menu - which items exist,
    /// and whether each is enabled - is handled by action availability and
    /// needs no rebuild, which is why this compares before calling rather
    /// than replacing the bar on every tick.
    ///
    /// Only the active window rebuilds: the menu bar is app-wide, and two
    /// open workspace windows would otherwise overwrite each other's
    /// submenus on alternating polls.
    pub(in crate::workspace_window) fn refresh_agents_menu(&mut self, cx: &mut Context<Self>) {
        let active = cx.active_window() == Some(self.window_handle);
        let owned = cx.global::<AgentsMenuState>().owner == Some(self.window_handle);
        if !active && !owned {
            // The menu bar is showing someone else's selection, or nobody's.
            return;
        }
        // Becoming inactive hands the menu back rather than leaving this
        // window's submenus up behind the workspace manager, and only the
        // window holding it may do so - which is what makes the order the
        // two windows happen to poll in stop mattering.
        let (owner, snapshot) = if active {
            (Some(self.window_handle),
             self.selected_agent_menu(cx)
                 .map(|selected| selected.snapshot)
                 .unwrap_or_default())
        }
        else {
            (None, AgentMenuSnapshot::default())
        };
        let state = cx.global::<AgentsMenuState>();
        if state.owner == owner && state.snapshot == snapshot {
            return;
        }
        cx.set_global(AgentsMenuState { owner,
                                        snapshot: snapshot.clone() });
        app_bootstrap::set_app_menus(&snapshot, cx);
    }

    /// Everything the menu bar's Agents menu needs about the sidebar's
    /// selection - `None` when nothing is selected, which is what leaves
    /// every item in that menu disabled.
    ///
    /// Read here rather than when the menu opens, because macOS validates
    /// menu items against the *last rendered frame's* dispatch tree; the
    /// item set this returns is what decides which handlers that frame
    /// carries.
    pub(in crate::workspace_window) fn selected_agent_menu(&self, cx: &Context<Self>)
                                                           -> Option<SelectedAgentMenu> {
        let id = self.selected_agent?;
        let store = self.store.lock();
        let (name, folder) = {
            let agent = store.agent(id)?;
            (agent.name.clone(), agent.folder.clone())
        };
        let (facts, move_targets, markdown_history) = agent_menu_facts(&store, id);
        drop(store);
        Some(SelectedAgentMenu { targets:  AgentMenuTargets { store: Arc::clone(&self.store),
                                                              window_entity: cx.entity(),
                                                              workspace_id: self.workspace_id,
                                                              id,
                                                              name,
                                                              folder },
                                 snapshot: AgentMenuSnapshot { entries:
                                                                   agent_context_menu_entries(facts),
                                                               move_targets,
                                                               markdown_history }, })
    }

    pub(in crate::workspace_window) fn selected_agent_header(&self) -> Option<SelectedAgentHeader> {
        let id = self.selected_agent?;
        let store = self.store.lock();
        let agent = store.agent(id)?;
        let stats = self.diff_stats.get(&id);
        let state = (!agent.is_shell()).then_some((agent.state, stats));
        Some(SelectedAgentHeader { avatar: agent.avatar.clone(),
                                   name: agent.name.clone(),
                                   folder: shorten_path(&agent.folder),
                                   header_title: agent.header_title().to_string(),
                                   agent_type: agent.agent_type.clone(),
                                   state })
    }
}

/// What the menu bar's Agents menu needs to know about the sidebar's
/// selection, gathered in one pass over the store.
pub(in crate::workspace_window) struct SelectedAgentMenu {
    targets:  AgentMenuTargets,
    /// The items that apply to this agent, and what its submenus should
    /// list. An item absent from `snapshot.entries` gets no action handler,
    /// which is what makes macOS draw it disabled - see `agent_menu`'s
    /// module docs.
    snapshot: AgentMenuSnapshot,
}

/// The Agents menu's handlers, registered on the workspace window's root
/// element - in the focus path, and deliberately not global: a global
/// listener makes `App::is_action_available` true unconditionally, which
/// would leave every item enabled with no agent selected and no window
/// open.
pub(in crate::workspace_window) fn with_agents_menu_actions(el: gpui_kit::Div,
                                                            selected: Option<&SelectedAgentMenu>)
                                                            -> gpui_kit::Div {
    let el = agents_menu_handlers!(el,
                                   selected,
                                   [NewCompanion => AgentMenuNewCompanion,
                                    NewShellCompanion => AgentMenuNewShellCompanion,
                                    EditAgent => AgentMenuEditAgent,
                                    ForkAgent => AgentMenuForkAgent,
                                    DuplicateAgent => AgentMenuDuplicateAgent,
                                    MoveToWorkspace => AgentMenuMoveToWorkspace,
                                    SaveToBench => AgentMenuSaveToBench,
                                    OpenIn => AgentMenuOpenIn,
                                    MarkdownFiles => AgentMenuMarkdownFiles,
                                    RegisterAgent => AgentMenuRegisterAgent,
                                    Deactivate => AgentMenuDeactivate,
                                    RestartAgent => AgentMenuRestartAgent,
                                    RestartWithNewConversation => AgentMenuRestartWithNewConversation,
                                    RemoveAgent => AgentMenuRemoveAgent]);
    let Some(selected) = selected
    else {
        return el;
    };
    // The submenu leaves. Each carries its own payload, so one handler
    // covers a whole submenu; registering it only when the parent entry
    // applies is what disables the leaves - and with them the parent item,
    // which AppKit enables only when a child is enabled.
    let el = if selected.snapshot
                        .entries
                        .contains(&AgentMenuEntry::MoveToWorkspace)
    {
        let targets = selected.targets.clone();
        el.on_action(move |action: &AgentMenuMoveToWorkspaceTarget, _window, app| {
                         move_agent_to_workspace(&targets, action.workspace_id, app);
                     })
    }
    else {
        el
    };
    let el = if selected.snapshot.entries.contains(&AgentMenuEntry::OpenIn) {
        let folder = selected.targets.folder.clone();
        el.on_action(move |action: &AgentMenuOpenInApp, _window, _app| {
              if let Some(app) = open_in::OpenInApp::from_id(action.app_id.as_ref()) {
                  open_in::open_folder(app, &folder);
              }
          })
    }
    else {
        el
    };
    if selected.snapshot
               .entries
               .contains(&AgentMenuEntry::MarkdownFiles)
    {
        let targets = selected.targets.clone();
        el.on_action(move |action: &AgentMenuShowMarkdownFile, _window, app| {
              show_agent_markdown_file(&targets, &action.path, app);
          })
    }
    else {
        el
    }
}
