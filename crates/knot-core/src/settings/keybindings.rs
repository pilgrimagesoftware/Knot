//! The user's customizations of the navigation shortcuts.
//!
//! Contract: `openspec/specs/keybindings/spec.md`.
//!
//! Only a customization is stored: every field is `None` until the user
//! changes that shortcut, and `None` means "the default". A default that
//! changes in a later release therefore reaches everyone who never touched
//! it. Chords are gpui keystroke strings (`"cmd-alt-p"`) held as text,
//! because parsing them is gpui's business and this crate does not depend on
//! gpui; the app validates them and falls back to the default for one that
//! does not parse.
//!
//! Decoding is lenient field by field. The preferences document is decoded
//! as one value and falls back to the defaults *wholesale* when any field
//! fails, so a hand-edited shortcut of the wrong type would otherwise reset
//! every preference the user has.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// A modifier combination for one of the numbered shortcut families, whose
/// key is always a digit 1-9.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShortcutModifiers {
    pub command: bool,
    pub control: bool,
    pub alt:     bool,
    pub shift:   bool,
}

/// The customized shortcuts. See the module docs for what `None` means.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeybindingSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_select_modifiers: Option<ShortcutModifiers>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_select_modifiers:     Option<ShortcutModifiers>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_input:                Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toggle_dashboard:           Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toggle_pull_requests:       Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_command_center:        Option<String>,
}

impl<'de> Deserialize<'de> for KeybindingSettings {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let field = |name: &str| value.get(name).cloned();
        let modifiers = |name: &str| {
            field(name).and_then(|v| serde_json::from_value::<ShortcutModifiers>(v).ok())
        };
        let chord = |name: &str| field(name).and_then(|v| v.as_str().map(str::to_string));
        Ok(Self { workspace_select_modifiers: modifiers("workspaceSelectModifiers"),
                  agent_select_modifiers:     modifiers("agentSelectModifiers"),
                  focus_input:                chord("focusInput"),
                  toggle_dashboard:           chord("toggleDashboard"),
                  toggle_pull_requests:       chord("togglePullRequests"),
                  open_command_center:        chord("openCommandCenter"), })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_uncustomized_value_serializes_to_an_empty_object() {
        let value = serde_json::to_value(KeybindingSettings::default()).unwrap();
        assert_eq!(value, serde_json::json!({}));
    }

    #[test]
    fn a_customization_round_trips() {
        let settings = KeybindingSettings { agent_select_modifiers:
                                                Some(ShortcutModifiers { command: true,
                                                                         control: true,
                                                                         ..Default::default() }),
                                            toggle_dashboard: Some("cmd-ctrl-k".into()),
                                            ..Default::default() };
        let value = serde_json::to_value(&settings).unwrap();
        assert_eq!(serde_json::from_value::<KeybindingSettings>(value).unwrap(),
                   settings);
    }

    #[test]
    fn a_wrongly_typed_field_is_dropped_rather_than_failing_the_rest() {
        let value = serde_json::json!({
            "focusInput": 7,
            "agentSelectModifiers": "cmd",
            "togglePullRequests": "cmd-alt-u",
            "somethingNew": true,
        });
        let settings: KeybindingSettings = serde_json::from_value(value).unwrap();
        assert_eq!(settings,
                   KeybindingSettings { toggle_pull_requests: Some("cmd-alt-u".into()),
                                        ..Default::default() });
    }
}
