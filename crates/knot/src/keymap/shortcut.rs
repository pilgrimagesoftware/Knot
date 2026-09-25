//! The six configurable shortcuts, as a closed vocabulary.

use knot_core::ShortcutModifiers;

use crate::keymap::Chord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shortcut {
    SelectWorkspace,
    SelectAgent,
    FocusAgentInput,
    ToggleDashboard,
    TogglePullRequests,
    OpenCommandCenter,
}

impl Shortcut {
    /// In the order the Keyboard settings pane lists them.
    pub(crate) const ALL: [Shortcut; 6] = [Shortcut::SelectWorkspace,
                                           Shortcut::SelectAgent,
                                           Shortcut::FocusAgentInput,
                                           Shortcut::ToggleDashboard,
                                           Shortcut::TogglePullRequests,
                                           Shortcut::OpenCommandCenter];

    pub(crate) fn label_key(self) -> &'static str {
        match self {
            Shortcut::SelectWorkspace => "keymap.shortcut.select_workspace",
            Shortcut::SelectAgent => "keymap.shortcut.select_agent",
            Shortcut::FocusAgentInput => "keymap.shortcut.focus_agent_input",
            Shortcut::ToggleDashboard => "keymap.shortcut.toggle_dashboard",
            Shortcut::TogglePullRequests => "keymap.shortcut.toggle_pull_requests",
            Shortcut::OpenCommandCenter => "keymap.shortcut.open_command_center",
        }
    }

    pub(crate) fn label(self) -> String {
        knot_core::l10n::t(self.label_key())
    }

    /// A family's default modifier. `None` for a single-chord shortcut.
    pub(crate) fn default_modifiers(self) -> Option<ShortcutModifiers> {
        match self {
            // The Swift reference's workspace switcher (`SkwadApp.swift`).
            Shortcut::SelectWorkspace => Some(ShortcutModifiers { command: true,
                                                                  ..Default::default() }),
            Shortcut::SelectAgent => Some(ShortcutModifiers { command: true,
                                                              alt: true,
                                                              ..Default::default() }),
            _ => None,
        }
    }

    /// A single-chord shortcut's default. `None` for a family.
    pub(crate) fn default_chord(self) -> Option<Chord> {
        let source = match self {
            Shortcut::FocusAgentInput => "cmd-l",
            // cmd-alt-d, the obvious one, is macOS's Dock toggle.
            Shortcut::ToggleDashboard => "cmd-alt-o",
            Shortcut::TogglePullRequests => "cmd-alt-p",
            Shortcut::OpenCommandCenter => "cmd-alt-0",
            Shortcut::SelectWorkspace | Shortcut::SelectAgent => return None,
        };
        Chord::parse(source)
    }
}
