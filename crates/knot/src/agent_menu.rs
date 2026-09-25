//! The menu bar's **Agents** menu: the actions it dispatches, and how it is
//! built from the same item set as the agent row's context menu.
//!
//! Contract: `openspec/specs/app-menu/spec.md`.
//!
//! The two menus diverge in one place, deliberately. The context menu omits
//! what does not apply to the row it was opened on; this menu always shows
//! every item and lets macOS grey out what the selected agent cannot do. A
//! menu bar is navigated from memory by position, so an item set that
//! changes shape between selections cannot be learned.
//!
//! Enablement of the plain items is not written into the [`Menu`] built
//! here. macOS asks gpui per item through `validate_menu_item` ->
//! [`gpui_kit::App::is_action_available`], which answers from the last rendered
//! frame's dispatch tree: the workspace window registers a handler for exactly
//! the entries [`agent_context_menu_entries`] produces for its selected agent,
//! and the items with no handler draw disabled.
//!
//! *This is why those actions must never be registered globally.*
//! `App::is_action_available` ORs the focused window's answer with "is there
//! a global listener", so one global registration would leave every item
//! permanently enabled - including with no window open at all.
//!
//! The three submenu items are the exception, and have to be disabled here.
//! AppKit only validates menu items that carry an action, and a submenu's
//! parent carries none, so it draws enabled however unavailable its leaves
//! are - verified against the running app, not assumed. Their state
//! therefore comes from [`AgentMenuSnapshot::entries`], which is also what
//! makes them follow the active window: see
//! `WorkspaceWindow::refresh_agents_menu`.

use std::path::Path;
use std::path::PathBuf;

use gpui_kit::KeyBinding;
use gpui_kit::Menu;
use gpui_kit::MenuItem;
use gpui_kit::actions;
use gpui_kit::{Action, SharedString};
use uuid::Uuid;

use crate::app_state::AgentMenuEntry;
use crate::app_state::AgentMenuFacts;
use crate::app_state::agent_context_menu_entries;
use crate::open_in;

actions!(knot_app,
         [AgentMenuNewCompanion,
          AgentMenuNewShellCompanion,
          AgentMenuEditAgent,
          AgentMenuForkAgent,
          AgentMenuDuplicateAgent,
          AgentMenuMoveToWorkspace,
          AgentMenuSaveToBench,
          AgentMenuOpenIn,
          AgentMenuMarkdownFiles,
          AgentMenuRegisterAgent,
          AgentMenuDeactivate,
          AgentMenuRestartAgent,
          AgentMenuRestartWithNewConversation,
          AgentMenuRemoveAgent]);

/// The shortcuts the Swift reference gives to items that live in this
/// menu, which are spread across its File and Edit command groups rather
/// than sitting on its context menu (`Skwad/SkwadApp.swift:198`, `:277`,
/// `:287`, `:295`).
///
/// The reference's four others do not land here: New Agent (cmd-t) and
/// Broadcast (cmd-shift-b) belong to the sidebar's background menu, which
/// has no menu-bar presence to show a shortcut on; Open in <app>
/// (cmd-shift-o) was one item for the *default* application, where this
/// port has a submenu of all of them; and Close Agent (cmd-w) is this
/// menu's Remove Agent, which keeps no shortcut because cmd-w is Close
/// Window here.
///
/// Fork Agent departs from the reference's cmd-f, which is Find on every
/// other Mac application: gpui binds it to in-field Search already, and a
/// menu item's key equivalent is claimed by AppKit ahead of the window, so
/// taking it would spend the platform's find key on a Knot action before
/// this port has a find of its own to put there. cmd-alt-f is the nearest
/// free key.
///
/// No context, so a workspace window answers these wherever focus sits
/// inside it. The handlers are registered per selected agent on the
/// window's root element, so these do nothing at all when no agent is
/// selected - the same rule that greys the menu items.
pub(crate) fn agent_menu_key_bindings() -> Vec<KeyBinding> {
    vec![KeyBinding::new("cmd-shift-s", AgentMenuNewShellCompanion, None),
         KeyBinding::new("cmd-alt-f", AgentMenuForkAgent, None),
         KeyBinding::new("cmd-d", AgentMenuDuplicateAgent, None),
         KeyBinding::new("cmd-r", AgentMenuRestartAgent, None),
         // Knot's own: the reference has no such item. Restart Agent's key
         // with Shift, since it is that item's other half (`app-menu`).
         KeyBinding::new("cmd-shift-r", AgentMenuRestartWithNewConversation, None)]
}

/// One workspace in the Move to Workspace submenu.
///
/// The dynamic submenus need a payload the unit actions above cannot carry.
/// One handler covers every leaf, because the dispatch tree matches on the
/// action's type and hands the payload to it.
#[derive(Clone, PartialEq, Eq, Debug, Action)]
#[action(namespace = knot_app, no_json)]
pub(crate) struct AgentMenuMoveToWorkspaceTarget {
    pub(crate) workspace_id: Uuid,
}

/// One application in the Open In… submenu.
#[derive(Clone, PartialEq, Eq, Debug, Action)]
#[action(namespace = knot_app, no_json)]
pub(crate) struct AgentMenuOpenInApp {
    pub(crate) app_id: SharedString,
}

/// One file in the Markdown Files submenu.
#[derive(Clone, PartialEq, Eq, Debug, Action)]
#[action(namespace = knot_app, no_json)]
pub(crate) struct AgentMenuShowMarkdownFile {
    pub(crate) path: PathBuf,
}

/// The action that invokes `entry`, or `None` for a separator.
///
/// Total over the labelled entries, and injective - the pairing is asserted
/// both ways in `tests::every_labelled_entry_has_exactly_one_menu_action`,
/// so an item added to the context menu cannot silently skip the menu bar.
pub(crate) fn agent_menu_action(entry: AgentMenuEntry) -> Option<Box<dyn Action>> {
    Some(match entry {
             AgentMenuEntry::Separator => return None,
             AgentMenuEntry::NewCompanion => Box::new(AgentMenuNewCompanion),
             AgentMenuEntry::NewShellCompanion => Box::new(AgentMenuNewShellCompanion),
             AgentMenuEntry::EditAgent => Box::new(AgentMenuEditAgent),
             AgentMenuEntry::ForkAgent => Box::new(AgentMenuForkAgent),
             AgentMenuEntry::DuplicateAgent => Box::new(AgentMenuDuplicateAgent),
             AgentMenuEntry::MoveToWorkspace => Box::new(AgentMenuMoveToWorkspace),
             AgentMenuEntry::SaveToBench => Box::new(AgentMenuSaveToBench),
             AgentMenuEntry::OpenIn => Box::new(AgentMenuOpenIn),
             AgentMenuEntry::MarkdownFiles => Box::new(AgentMenuMarkdownFiles),
             AgentMenuEntry::RegisterAgent => Box::new(AgentMenuRegisterAgent),
             AgentMenuEntry::Deactivate => Box::new(AgentMenuDeactivate),
             AgentMenuEntry::RestartAgent => Box::new(AgentMenuRestartAgent),
             AgentMenuEntry::RestartWithNewConversation => {
                 Box::new(AgentMenuRestartWithNewConversation)
             }
             AgentMenuEntry::RemoveAgent => Box::new(AgentMenuRemoveAgent),
         })
}

// The inverse of `action_for_agent_menu_entry`, unused since the menu
// bar started dispatching by entry rather than by action type.
#[allow(dead_code)]
/// The entry `action` invokes, or `None` for anything else - the other half
/// of [`agent_menu_action`]'s pairing.
pub(crate) fn agent_menu_entry_for_action(action: &dyn Action) -> Option<AgentMenuEntry> {
    AgentMenuEntry::ALL.into_iter().find(|entry| {
                                       agent_menu_action(*entry).is_some_and(|candidate| {
                                                                    candidate.partial_eq(action)
                                                                })
                                   })
}

/// What the Agents menu's dynamic submenus list. Read from the store for the
/// selected agent, and compared against the last one built to decide whether
/// the menu bar needs rebuilding - a [`gpui_kit::Menu`] is a static snapshot,
/// so submenu contents do not re-evaluate on their own.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AgentMenuSnapshot {
    /// The items that apply to the selected agent, which is what decides
    /// whether each submenu draws enabled. Empty when no agent is selected
    /// in the active window - and so are the submenus.
    pub(crate) entries:          Vec<AgentMenuEntry>,
    /// The workspaces the selected agent could move to, in menu order.
    pub(crate) move_targets:     Vec<(Uuid, String)>,
    /// The markdown files the selected agent has shown.
    pub(crate) markdown_history: Vec<PathBuf>,
}

/// Which window's selection the menu bar's Agents menu is currently
/// showing, and what it was built from.
///
/// The menu bar is app-wide but the selection it acts on belongs to one
/// window, so ownership has to be recorded somewhere both windows can see:
/// the active window claims it, and only the window that holds it may give
/// it up. Without that, two open workspace windows would overwrite each
/// other's submenus on alternating polls.
#[derive(Default)]
pub(crate) struct AgentsMenuState {
    pub(crate) owner:    Option<gpui_kit::AnyWindowHandle>,
    pub(crate) snapshot: AgentMenuSnapshot,
}

impl gpui_kit::Global for AgentsMenuState {}

/// The Agents menu, carrying every item the context menu can offer.
///
/// Built from [`agent_context_menu_entries`] with the facts of an agent that
/// every item applies to, which is what fixes the order and grouping to the
/// context menu's: the two menus cannot drift apart because they are the
/// same list.
pub(crate) fn agents_menu(snapshot: &AgentMenuSnapshot) -> Menu {
    let items = agent_context_menu_entries(AgentMenuFacts::EVERY_ITEM).into_iter()
                                                                      .filter_map(|entry| {
                                                                          agent_menu_item(entry,
                                                                                          snapshot)
                                                                      })
                                                                      .collect::<Vec<_>>();
    Menu::new(knot_core::l10n::t("menu.agents")).items(items)
}

/// One item of the Agents menu. The three submenu entries carry no action of
/// their own; everything else dispatches its unit action.
fn agent_menu_item(entry: AgentMenuEntry, snapshot: &AgentMenuSnapshot) -> Option<MenuItem> {
    Some(match entry {
             AgentMenuEntry::Separator => MenuItem::separator(),
             AgentMenuEntry::MoveToWorkspace
             | AgentMenuEntry::OpenIn
             | AgentMenuEntry::MarkdownFiles => {
                 agent_menu_submenu(entry, snapshot)?.disabled(!snapshot.entries.contains(&entry))
             }
             // Not `MenuItem::action`, which takes a concrete action:
             // `Box<dyn Action>` is what keeps the pairing in one table
             // rather than repeating the match here.
             entry => MenuItem::Action { name:      entry.label()?.into(),
                                         action:    agent_menu_action(entry)?,
                                         os_action: None,
                                         checked:   false,
                                         disabled:  false, },
         })
}

/// One of the three submenu items, contents and all.
///
/// The leaves carry their own payload actions and so are validated like any
/// other item; the parent is not, which is why its caller disables it from
/// the snapshot. An inapplicable submenu is emitted empty rather than
/// dropped - the item holds its position either way.
fn agent_menu_submenu(entry: AgentMenuEntry, snapshot: &AgentMenuSnapshot) -> Option<MenuItem> {
    let items: Vec<MenuItem> = match entry {
        AgentMenuEntry::MoveToWorkspace => {
            snapshot.move_targets
                    .iter()
                    .map(|(workspace_id, name)| {
                        MenuItem::action(name.clone(),
                                         AgentMenuMoveToWorkspaceTarget { workspace_id:
                                                                              *workspace_id, })
                    })
                    .collect()
        }
        // A fixed list: which applications exist does not depend on the
        // agent, only the folder each one is asked to open.
        AgentMenuEntry::OpenIn => {
            open_in::open_in_entries().into_iter()
                                      .map(|item| match item {
                                          open_in::OpenInEntry::Separator => MenuItem::separator(),
                                          open_in::OpenInEntry::App(app) => {
                                              MenuItem::action(app.label(),
                                                               AgentMenuOpenInApp { app_id:
                                                                                        app.id()
                                                                                           .into(), })
                                          }
                                      })
                                      .collect()
        }
        AgentMenuEntry::MarkdownFiles => {
            snapshot.markdown_history
                    .iter()
                    .map(|path| {
                        MenuItem::action(markdown_label(path),
                                         AgentMenuShowMarkdownFile { path: path.clone() })
                    })
                    .collect()
        }
        _ => return None,
    };
    Some(MenuItem::submenu(Menu::new(entry.label()?).items(items)))
}

/// How a markdown file is labelled in either menu.
pub(crate) fn markdown_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}
