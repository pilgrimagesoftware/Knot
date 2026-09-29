//! Whether a new session should be moved onto the mode its user options
//! ask for (#501), or else its type's default mode
//! (`knot_agent_launch::unconfigured_default_mode`, #516).
//!
//! Pure, so the rule is tested without an adapter.

use std::collections::BTreeMap;

use knot_acp::ConfigOption;
use knot_agent_launch::DefaultMode;

/// The mode to move a new session onto, if any: the one the agent's user
/// options ask for, whatever `persisted` holds - a CLI flag applies on every
/// launch, and so does its setting here - else `default`, per
/// [`default_mode_to_apply`].
pub(super) fn mode_to_apply(asked: Option<DefaultMode>, default: Option<DefaultMode>,
                            persisted: &BTreeMap<String, String>, options: &[ConfigOption])
                            -> Option<DefaultMode> {
    match asked {
        Some(asked) => offered_and_unset(asked, options),
        None => default_mode_to_apply(default, persisted, options),
    }
}

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
    offered_and_unset(default, options)
}

/// `mode`, if the adapter offers its value and the session is not already
/// on it.
fn offered_and_unset(mode: DefaultMode, options: &[ConfigOption]) -> Option<DefaultMode> {
    let option = options.iter().find(|option| option.id == mode.config_id)?;
    let offered = option.options
                        .iter()
                        .any(|choice| choice.value == mode.value);
    let current = option.current_value.as_str() == Some(mode.value);
    (offered && !current).then_some(mode)
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

    const BYPASS: DefaultMode = DefaultMode { config_id: "mode",
                                              value:     "bypassPermissions", };

    #[test]
    fn an_asked_mode_wins_over_a_persisted_one() {
        let options = [mode_option("plan", &["default", "plan", "bypassPermissions"])];
        let persisted = BTreeMap::from([("mode".to_owned(), "plan".to_owned())]);
        assert_eq!(mode_to_apply(Some(BYPASS), Some(AUTO), &persisted, &options),
                   Some(BYPASS));
    }

    #[test]
    fn an_asked_mode_the_adapter_lacks_is_not_requested() {
        // `claude-agent-acp` drops bypass from its catalog when run as root.
        let options = [mode_option("default", &["default", "auto"])];
        assert_eq!(mode_to_apply(Some(BYPASS), Some(AUTO), &BTreeMap::new(), &options),
                   None);
    }

    #[test]
    fn without_an_asked_mode_the_default_applies() {
        let options = [mode_option("default", &["default", "auto"])];
        assert_eq!(mode_to_apply(None, Some(AUTO), &BTreeMap::new(), &options),
                   Some(AUTO));
    }

    #[test]
    fn no_default_changes_nothing() {
        let options = [mode_option("default", &["default", "auto"])];
        assert_eq!(default_mode_to_apply(None, &BTreeMap::new(), &options),
                   None);
    }
}
