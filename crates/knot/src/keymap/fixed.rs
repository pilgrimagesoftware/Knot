//! The shortcuts the user cannot rebind, each with the name a rejected
//! customization quotes back.
//!
//! Bootstrap binds from [`fixed_bindings`] rather than from lists of its own,
//! so a fixed shortcut cannot be added without the validator seeing it.

use gpui_kit::KeyBinding;

use crate::agent_menu::AgentMenuDuplicateAgent;
use crate::agent_menu::AgentMenuForkAgent;
use crate::agent_menu::AgentMenuNewShellCompanion;
use crate::agent_menu::AgentMenuRestartAgent;
use crate::agent_menu::AgentMenuRestartWithNewConversation;
use crate::agent_menu::agent_menu_key_bindings;
use crate::app_bootstrap::CloseWindow;
use crate::app_bootstrap::HideApp;
use crate::app_bootstrap::HideOthers;
use crate::app_bootstrap::KnotHelp;
use crate::app_bootstrap::MinimizeWindow;
use crate::app_bootstrap::NewAgent;
use crate::app_bootstrap::NewWorkspace;
use crate::app_bootstrap::OpenSettings;
use crate::app_bootstrap::OpenWorkspaces;
use crate::app_bootstrap::PanelOpenPermissionSelector;
use crate::app_bootstrap::PanelPermissionAllow;
use crate::app_bootstrap::PanelPermissionDeny;
use crate::app_bootstrap::Quit;
use crate::keymap::Chord;

/// A fixed binding and the l10n key of what it does.
pub(crate) struct FixedBinding {
    pub(crate) binding:   KeyBinding,
    pub(crate) label_key: &'static str,
}

/// Every fixed binding Knot installs.
pub(crate) fn fixed_bindings() -> Vec<FixedBinding> {
    let fixed = |binding, label_key| FixedBinding { binding, label_key };
    let mut out = vec![// The standard macOS application-menu shortcuts. A
                       // `MenuItem::action` only shows a shortcut next to its label if the
                       // action has a binding, so without these the menu read as if Knot
                       // had none.
                       fixed(KeyBinding::new("cmd-q", Quit, None), "keymap.fixed.quit"),
                       fixed(KeyBinding::new("cmd-,", OpenSettings, None),
                             "keymap.fixed.settings"),
                       fixed(KeyBinding::new("cmd-h", HideApp, None), "keymap.fixed.hide"),
                       fixed(KeyBinding::new("cmd-alt-h", HideOthers, None),
                             "keymap.fixed.hide_others"),
                       // The rest of the shortcuts macOS expects on a standard menu item,
                       // whether or not the item behind each is wired up yet - the menu bar
                       // reads as an app with no keyboard at all without them. About Knot,
                       // Show All and Zoom are absent on purpose: macOS gives those three no
                       // key equivalent either.
                       fixed(KeyBinding::new("cmd-n", NewWorkspace, None),
                             "keymap.fixed.new_workspace"),
                       // The Swift reference's New Agent key.
                       fixed(KeyBinding::new(crate::consts::NEW_AGENT_CHORD, NewAgent, None),
                             "keymap.fixed.new_agent"),
                       fixed(KeyBinding::new("cmd-w", CloseWindow, None),
                             "keymap.fixed.close_window"),
                       fixed(KeyBinding::new("cmd-m", MinimizeWindow, None),
                             "keymap.fixed.minimize"),
                       fixed(KeyBinding::new("cmd-shift-/", KnotHelp, None),
                             "keymap.fixed.help"),
                       // The workspace manager's opener. macOS does not reserve cmd-0: it
                       // is conventionally "reset zoom", and Knot has no zoom level for it
                       // to reset. The Command Center's opener is configurable, so it is
                       // not here.
                       fixed(KeyBinding::new("cmd-0", OpenWorkspaces, None),
                             "menu.window.workspaces"),
                       fixed(KeyBinding::new("cmd-shift-a", PanelPermissionAllow, None),
                             "keymap.fixed.permission_allow"),
                       fixed(KeyBinding::new("cmd-shift-d", PanelPermissionDeny, None),
                             "keymap.fixed.permission_deny"),
                       fixed(KeyBinding::new("cmd-shift-p", PanelOpenPermissionSelector, None),
                             "keymap.fixed.permission_selector"),];
    // The Agents menu's keys keep their definition beside the menu; see
    // `agent_menu::agent_menu_key_bindings`.
    out.extend(agent_menu_key_bindings().into_iter().map(|binding| {
                                                        let label_key =
                                                            agent_menu_label_key(&binding);
                                                        fixed(binding, label_key)
                                                    }));
    out
}

/// Chords Knot does not bind itself but must not take: the Edit menu's,
/// which gpui binds for a focused text field. A context-less binding of ours
/// would lose to those inside a field and win everywhere else, so the key
/// would do two different things.
pub(crate) fn reserved_chords() -> Vec<(Chord, &'static str)> {
    [("cmd-z", "keymap.fixed.undo"),
     ("cmd-shift-z", "keymap.fixed.redo"),
     ("cmd-x", "keymap.fixed.cut"),
     ("cmd-c", "keymap.fixed.copy"),
     ("cmd-v", "keymap.fixed.paste"),
     ("cmd-a", "keymap.fixed.select_all")].into_iter()
                                          .filter_map(|(source, key)| {
                                              Chord::parse(source).map(|chord| (chord, key))
                                          })
                                          .collect()
}

/// Every chord a configurable shortcut may not take, with what holds it.
pub(crate) fn taken_chords() -> Vec<(Chord, &'static str)> {
    let mut out: Vec<(Chord, &'static str)> =
        fixed_bindings().iter()
                        .filter_map(|fixed| {
                            Chord::from_binding(&fixed.binding).map(|chord| {
                                                                   (chord, fixed.label_key)
                                                               })
                        })
                        .collect();
    out.extend(reserved_chords());
    out
}

fn agent_menu_label_key(binding: &KeyBinding) -> &'static str {
    let action = binding.action().as_any();
    if action.is::<AgentMenuNewShellCompanion>() {
        "menu.agent.new_shell_companion"
    }
    else if action.is::<AgentMenuForkAgent>() {
        "menu.agent.fork_agent"
    }
    else if action.is::<AgentMenuDuplicateAgent>() {
        "menu.agent.duplicate_agent"
    }
    else if action.is::<AgentMenuRestartAgent>() {
        "menu.agent.restart_agent"
    }
    else if action.is::<AgentMenuRestartWithNewConversation>() {
        "menu.agent.restart_new_conversation"
    }
    else {
        "menu.agents"
    }
}
