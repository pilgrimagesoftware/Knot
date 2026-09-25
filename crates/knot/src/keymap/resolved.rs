//! The effective set of configurable shortcuts, and its conversion to and
//! from what the preferences document stores.

use gpui_kit::KeyBinding;
use knot_core::KeybindingSettings;
use knot_core::ShortcutModifiers;

use crate::app_bootstrap::OpenCommandCenter;
use crate::keymap::BindFn;
use crate::keymap::Chord;
use crate::keymap::FocusAgentInput;
use crate::keymap::JumpToBottom;
use crate::keymap::SELECT_AGENT;
use crate::keymap::SELECT_WORKSPACE;
use crate::keymap::Shortcut;
use crate::keymap::ToggleDashboard;
use crate::keymap::TogglePullRequests;
use crate::keymap::validate::validate;

/// One binding the configurable set installs.
pub(crate) struct ConfiguredBinding {
    pub(crate) chord: Chord,
    pub(crate) bind:  BindFn,
}

impl ConfiguredBinding {
    pub(crate) fn key_binding(&self) -> KeyBinding {
        (self.bind)(&self.chord.to_gpui())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resolved {
    workspace_modifiers:  ShortcutModifiers,
    agent_modifiers:      ShortcutModifiers,
    focus_agent_input:    Chord,
    toggle_dashboard:     Chord,
    toggle_pull_requests: Chord,
    open_command_center:  Chord,
    jump_to_bottom:       Chord,
}

/// A default that does not parse is a bug in `Shortcut::default_chord`,
/// which the keymap tests catch.
fn default_chord(shortcut: Shortcut) -> Chord {
    shortcut.default_chord()
            .expect("every single-chord shortcut has a parseable default")
}

fn default_modifiers(shortcut: Shortcut) -> ShortcutModifiers {
    shortcut.default_modifiers()
            .expect("every family has a default modifier")
}

impl Resolved {
    pub(crate) fn defaults() -> Self {
        Self { workspace_modifiers:  default_modifiers(Shortcut::SelectWorkspace),
               agent_modifiers:      default_modifiers(Shortcut::SelectAgent),
               focus_agent_input:    default_chord(Shortcut::FocusAgentInput),
               toggle_dashboard:     default_chord(Shortcut::ToggleDashboard),
               toggle_pull_requests: default_chord(Shortcut::TogglePullRequests),
               open_command_center:  default_chord(Shortcut::OpenCommandCenter),
               jump_to_bottom:       default_chord(Shortcut::JumpToBottom), }
    }

    /// The stored customizations applied over the defaults, one at a time
    /// and each validated against those already accepted. One that does not
    /// parse, or that [`validate`] rejects, is skipped and its default kept
    /// (`keybindings`: "A customization cannot break other shortcuts").
    pub(crate) fn from_settings(stored: &KeybindingSettings) -> Self {
        let mut resolved = Self::defaults();
        let families = [(Shortcut::SelectWorkspace, stored.workspace_select_modifiers),
                        (Shortcut::SelectAgent, stored.agent_select_modifiers)];
        for (shortcut, modifiers) in families {
            if let Some(modifiers) = modifiers {
                resolved = resolved.accept(shortcut, |r| r.with_modifiers(shortcut, modifiers));
            }
        }
        let singles = [(Shortcut::FocusAgentInput, &stored.focus_input),
                       (Shortcut::ToggleDashboard, &stored.toggle_dashboard),
                       (Shortcut::TogglePullRequests, &stored.toggle_pull_requests),
                       (Shortcut::OpenCommandCenter, &stored.open_command_center),
                       (Shortcut::JumpToBottom, &stored.jump_to_bottom)];
        for (shortcut, source) in singles {
            let Some(source) = source
            else {
                continue;
            };
            match Chord::parse(source) {
                Some(chord) => {
                    resolved = resolved.accept(shortcut, |r| r.with_chord(shortcut, chord))
                }
                None => {
                    eprintln!("knot: ignoring the stored {} shortcut: `{source}` does not parse",
                              shortcut.label());
                }
            }
        }
        resolved
    }

    /// `change` applied to `self` if the result validates, else `self`.
    fn accept(self, shortcut: Shortcut, change: impl FnOnce(Self) -> Self) -> Self {
        let candidate = change(self.clone());
        match validate(&candidate, shortcut) {
            Ok(()) => candidate,
            Err(rejection) => {
                eprintln!("knot: ignoring the stored {} shortcut: {}",
                          shortcut.label(),
                          rejection.message());
                self
            }
        }
    }

    /// Only what differs from the default is stored, so a default that
    /// changes later reaches everyone who never customized it.
    pub(crate) fn to_settings(&self) -> KeybindingSettings {
        let base = Self::defaults();
        let modifiers = |shortcut| {
            let current = self.modifiers(shortcut);
            (current != base.modifiers(shortcut)).then_some(current)
                                                 .flatten()
        };
        let chord = |shortcut| {
            let current = self.chord(shortcut);
            (current != base.chord(shortcut)).then(|| current.map(Chord::to_gpui))
                                             .flatten()
        };
        KeybindingSettings { workspace_select_modifiers: modifiers(Shortcut::SelectWorkspace),
                             agent_select_modifiers:     modifiers(Shortcut::SelectAgent),
                             focus_input:                chord(Shortcut::FocusAgentInput),
                             toggle_dashboard:           chord(Shortcut::ToggleDashboard),
                             toggle_pull_requests:       chord(Shortcut::TogglePullRequests),
                             open_command_center:        chord(Shortcut::OpenCommandCenter),
                             jump_to_bottom:             chord(Shortcut::JumpToBottom), }
    }

    pub(crate) fn modifiers(&self, shortcut: Shortcut) -> Option<ShortcutModifiers> {
        match shortcut {
            Shortcut::SelectWorkspace => Some(self.workspace_modifiers),
            Shortcut::SelectAgent => Some(self.agent_modifiers),
            _ => None,
        }
    }

    pub(crate) fn chord(&self, shortcut: Shortcut) -> Option<&Chord> {
        match shortcut {
            Shortcut::FocusAgentInput => Some(&self.focus_agent_input),
            Shortcut::ToggleDashboard => Some(&self.toggle_dashboard),
            Shortcut::TogglePullRequests => Some(&self.toggle_pull_requests),
            Shortcut::OpenCommandCenter => Some(&self.open_command_center),
            Shortcut::JumpToBottom => Some(&self.jump_to_bottom),
            Shortcut::SelectWorkspace | Shortcut::SelectAgent => None,
        }
    }

    /// Sets a family's modifier. No effect on a single-chord shortcut.
    pub(crate) fn with_modifiers(mut self, shortcut: Shortcut, modifiers: ShortcutModifiers)
                                 -> Self {
        match shortcut {
            Shortcut::SelectWorkspace => self.workspace_modifiers = modifiers,
            Shortcut::SelectAgent => self.agent_modifiers = modifiers,
            _ => {}
        }
        self
    }

    /// Sets a single-chord shortcut. No effect on a family.
    pub(crate) fn with_chord(mut self, shortcut: Shortcut, chord: Chord) -> Self {
        match shortcut {
            Shortcut::FocusAgentInput => self.focus_agent_input = chord,
            Shortcut::ToggleDashboard => self.toggle_dashboard = chord,
            Shortcut::TogglePullRequests => self.toggle_pull_requests = chord,
            Shortcut::OpenCommandCenter => self.open_command_center = chord,
            Shortcut::JumpToBottom => self.jump_to_bottom = chord,
            Shortcut::SelectWorkspace | Shortcut::SelectAgent => {}
        }
        self
    }

    /// Puts one shortcut back to its default.
    pub(crate) fn reset(self, shortcut: Shortcut) -> Self {
        let base = Self::defaults();
        match (base.modifiers(shortcut), base.chord(shortcut)) {
            (Some(modifiers), _) => self.with_modifiers(shortcut, modifiers),
            (_, Some(chord)) => self.with_chord(shortcut, chord.clone()),
            (None, None) => self,
        }
    }

    /// Every chord `shortcut` answers to: nine for a family, one otherwise.
    pub(crate) fn chords_of(&self, shortcut: Shortcut) -> Vec<Chord> {
        match self.modifiers(shortcut) {
            Some(modifiers) => (1..=9).map(|digit| Chord::new(modifiers, &digit.to_string()))
                                      .collect(),
            None => self.chord(shortcut).cloned().into_iter().collect(),
        }
    }

    /// Every binding this set installs.
    pub(crate) fn bindings(&self) -> Vec<ConfiguredBinding> {
        let mut out = Vec::new();
        for (shortcut, binders) in [(Shortcut::SelectWorkspace, SELECT_WORKSPACE),
                                    (Shortcut::SelectAgent, SELECT_AGENT)]
        {
            for (chord, bind) in self.chords_of(shortcut).into_iter().zip(binders) {
                out.push(ConfiguredBinding { chord, bind });
            }
        }
        let singles: [(Shortcut, BindFn); 5] =
            [(Shortcut::FocusAgentInput, |c| KeyBinding::new(c, FocusAgentInput, None)),
             (Shortcut::ToggleDashboard, |c| KeyBinding::new(c, ToggleDashboard, None)),
             (Shortcut::TogglePullRequests, |c| KeyBinding::new(c, TogglePullRequests, None)),
             (Shortcut::OpenCommandCenter, |c| KeyBinding::new(c, OpenCommandCenter, None)),
             (Shortcut::JumpToBottom, |c| KeyBinding::new(c, JumpToBottom, None))];
        for (shortcut, bind) in singles {
            for chord in self.chords_of(shortcut) {
                out.push(ConfiguredBinding { chord, bind });
            }
        }
        out
    }
}
