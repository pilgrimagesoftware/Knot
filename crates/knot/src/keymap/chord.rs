//! One keystroke with modifiers, in the two forms it needs: gpui's binding
//! syntax (`"cmd-alt-p"`) and the glyphs the user reads (`⌥⌘P`).

use gpui_kit::Keystroke;
use knot_core::ShortcutModifiers;

/// A single modified keystroke. Two chords are equal when their modifiers
/// and key are, whatever order the source string named the modifiers in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Chord {
    pub(crate) modifiers: ShortcutModifiers,
    /// gpui's key name, lower case: `"p"`, `"1"`, `"space"`.
    pub(crate) key:       String,
}

impl Chord {
    pub(crate) fn new(modifiers: ShortcutModifiers, key: &str) -> Self {
        Self { modifiers,
               key: key.to_lowercase() }
    }

    /// Parses gpui binding syntax. A sequence (`"cmd-k cmd-s"`) and a chord
    /// using the `fn` modifier are refused: neither can be recorded here, so
    /// a stored one is a hand edit.
    pub(crate) fn parse(source: &str) -> Option<Self> {
        if source.split_whitespace().count() != 1 {
            return None;
        }
        let keystroke = Keystroke::parse(source.trim()).ok()?;
        if keystroke.modifiers.function {
            return None;
        }
        Some(Self::from_keystroke(&keystroke))
    }

    pub(crate) fn from_keystroke(keystroke: &Keystroke) -> Self {
        let m = &keystroke.modifiers;
        Self::new(ShortcutModifiers { command: m.platform,
                                      control: m.control,
                                      alt:     m.alt,
                                      shift:   m.shift, },
                  &keystroke.key)
    }

    /// The chord a single-keystroke binding answers to; `None` for a
    /// sequence.
    pub(crate) fn from_binding(binding: &gpui_kit::KeyBinding) -> Option<Self> {
        match binding.keystrokes() {
            [only] => Some(Self::from_keystroke(only.inner())),
            _ => None,
        }
    }

    /// gpui binding syntax, modifiers in a fixed order.
    pub(crate) fn to_gpui(&self) -> String {
        let m = &self.modifiers;
        let mut out = String::new();
        for (on, name) in [(m.control, "ctrl-"),
                           (m.alt, "alt-"),
                           (m.shift, "shift-"),
                           (m.command, "cmd-")]
        {
            if on {
                out.push_str(name);
            }
        }
        out.push_str(&self.key);
        out
    }

    /// Whether the chord holds ⌘, ⌃ or ⌥. Without one it would take a key
    /// away from typing.
    pub(crate) fn has_primary_modifier(&self) -> bool {
        has_primary_modifier(self.modifiers)
    }

    /// The glyph form macOS menus use: `⌃⌥⇧⌘` in that order, then the key.
    pub(crate) fn label(&self) -> String {
        format!("{}{}",
                modifiers_label(self.modifiers),
                key_label(&self.key))
    }
}

pub(crate) fn has_primary_modifier(modifiers: ShortcutModifiers) -> bool {
    modifiers.command || modifiers.control || modifiers.alt
}

/// `⌃⌥⇧⌘` glyphs for the modifiers that are held, in macOS order.
pub(crate) fn modifiers_label(modifiers: ShortcutModifiers) -> String {
    [(modifiers.control, '⌃'),
     (modifiers.alt, '⌥'),
     (modifiers.shift, '⇧'),
     (modifiers.command, '⌘')].iter()
                              .filter(|(on, _)| *on)
                              .map(|(_, glyph)| *glyph)
                              .collect()
}

fn key_label(key: &str) -> String {
    match key {
        "space" => "Space".into(),
        "enter" => "↩".into(),
        "tab" => "⇥".into(),
        "backspace" => "⌫".into(),
        "delete" => "⌦".into(),
        "left" => "←".into(),
        "right" => "→".into(),
        "up" => "↑".into(),
        "down" => "↓".into(),
        other => other.to_uppercase(),
    }
}
