//! Unit tests for [`super`].

use super::*;

#[test]
fn every_variant_round_trips_through_its_stored_string() {
    for mode in AppearanceMode::ALL {
        assert_eq!(mode.as_str().parse::<AppearanceMode>(), Ok(*mode));
    }
    for provider in AiProvider::ALL {
        assert_eq!(provider.as_str().parse::<AiProvider>(), Ok(*provider));
    }
    for action in AutopilotAction::ALL {
        assert_eq!(action.as_str().parse::<AutopilotAction>(), Ok(*action));
    }
}

#[test]
fn the_stored_strings_are_the_ones_already_in_users_settings_files() {
    assert_eq!(AppearanceMode::Auto.as_str(), "auto");
    assert_eq!(AppearanceMode::System.as_str(), "system");
    assert_eq!(AppearanceMode::Light.as_str(), "light");
    assert_eq!(AppearanceMode::Dark.as_str(), "dark");
    assert_eq!(AiProvider::OpenAi.as_str(), "openai");
    assert_eq!(AiProvider::Anthropic.as_str(), "anthropic");
    assert_eq!(AiProvider::Google.as_str(), "google");
    assert_eq!(AutopilotAction::Mark.as_str(), "mark");
    assert_eq!(AutopilotAction::Ask.as_str(), "ask");
    assert_eq!(AutopilotAction::Continue.as_str(), "continue");
    assert_eq!(AutopilotAction::Custom.as_str(), "custom");
}

/// The defaults have to match what `consts.rs` promised while these were
/// strings, or an existing settings file changes meaning on upgrade.
#[test]
fn the_defaults_match_the_string_constants_they_replace() {
    assert_eq!(AppearanceMode::default().as_str(),
               crate::consts::APPEARANCE_MODE_DEFAULT);
    assert_eq!(AiProvider::default().as_str(),
               crate::consts::AI_PROVIDER_DEFAULT);
    assert_eq!(AutopilotAction::default().as_str(),
               crate::consts::AUTOPILOT_ACTION_DEFAULT);
}

/// The bug this change exists to remove: a corrupt value used to render
/// identically to the real default, with nothing able to tell them apart.
#[test]
fn an_unrecognized_value_falls_back_and_says_what_it_was() {
    let (mode, unknown) = AppearanceMode::from_stored("aut0");

    assert_eq!(mode, AppearanceMode::Auto);
    assert_eq!(unknown, Some(UnknownVariant("aut0".to_string())));
    assert!(unknown.unwrap().to_string().contains("aut0"));
}

#[test]
fn a_recognized_value_reports_nothing_to_complain_about() {
    let (mode, unknown) = AppearanceMode::from_stored("dark");

    assert_eq!(mode, AppearanceMode::Dark);
    assert_eq!(unknown, None);
}

/// One unreadable field must not fail the whole settings document - it holds
/// every agent, workspace and persona.
#[test]
fn deserializing_an_unknown_value_degrades_rather_than_erroring() {
    let mode: AppearanceMode = serde_json::from_str(r#""nonsense""#).unwrap();

    assert_eq!(mode, AppearanceMode::default());
}

#[test]
fn serializing_writes_the_stored_string_not_the_variant_name() {
    assert_eq!(serde_json::to_string(&AiProvider::OpenAi).unwrap(),
               r#""openai""#);
    assert_eq!(serde_json::to_string(&AutopilotAction::Continue).unwrap(),
               r#""continue""#);
}
