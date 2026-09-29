//! Whether a new session should be moved onto its type's default mode
//! (`knot_agent_launch::unconfigured_default_mode`, #516).
//!
//! Pure, so the rule is tested without an adapter.

use std::collections::BTreeMap;

use knot_acp::ConfigOption;
use knot_agent_launch::DefaultMode;

/// `default`, if the session should be switched to it: nothing persisted
/// chose that option, the adapter offers the value, and the session is not
/// already on it.
///
/// An adapter that does not offer the value - an older one without Auto, say
/// - keeps its own default rather than being asked for one it would reject.
pub(super) fn default_mode_to_apply(default: Option<DefaultMode>,
                                    persisted: &BTreeMap<String, String>,
                                    options: &[ConfigOption])
                                    -> Option<DefaultMode> {
    let default = default?;
    if persisted.contains_key(default.config_id) {
        return None;
    }
    let option = options.iter()
                        .find(|option| option.id == default.config_id)?;
    let offered = option.options
                        .iter()
                        .any(|choice| choice.value == default.value);
    let current = option.current_value.as_str() == Some(default.value);
    (offered && !current).then_some(default)
}

#[cfg(test)]
mod tests {
    use knot_acp::ConfigOptionValue;
    use serde_json::json;

    use super::*;

    const AUTO: DefaultMode = DefaultMode { config_id: "mode",
                                            value:     "auto", };

    fn mode_option(current: &str, offered: &[&str]) -> ConfigOption {
        ConfigOption { id:            "mode".to_owned(),
                       name:          "Mode".to_owned(),
                       category:      Some("mode".to_owned()),
                       kind:          "select".to_owned(),
                       current_value: json!(current),
                       options:       offered.iter()
                                             .map(|value| {
                                                 ConfigOptionValue { value:
                                                                         (*value).to_owned(),
                                                                     name:
                                                                         (*value).to_owned(),
                                                                     description: None, }
                                             })
                                             .collect(), }
    }

    #[test]
    fn a_fresh_manual_session_moves_to_auto() {
        let options = [mode_option("default", &["default", "auto", "plan"])];
        assert_eq!(default_mode_to_apply(Some(AUTO), &BTreeMap::new(), &options),
                   Some(AUTO));
    }

    #[test]
    fn a_persisted_choice_wins() {
        let options = [mode_option("default", &["default", "auto"])];
        let persisted = BTreeMap::from([("mode".to_owned(), "default".to_owned())]);
        assert_eq!(default_mode_to_apply(Some(AUTO), &persisted, &options),
                   None);
    }

    #[test]
    fn an_adapter_without_auto_is_left_alone() {
        let options = [mode_option("default", &["default", "plan"])];
        assert_eq!(default_mode_to_apply(Some(AUTO), &BTreeMap::new(), &options),
                   None);
    }

    #[test]
    fn a_session_already_in_auto_is_left_alone() {
        let options = [mode_option("auto", &["default", "auto"])];
        assert_eq!(default_mode_to_apply(Some(AUTO), &BTreeMap::new(), &options),
                   None);
    }

    #[test]
    fn no_default_changes_nothing() {
        let options = [mode_option("default", &["default", "auto"])];
        assert_eq!(default_mode_to_apply(None, &BTreeMap::new(), &options),
                   None);
    }
}
