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

#[test]
fn cost_tier_round_trips_through_its_stored_string() {
    for tier in CostTier::ALL {
        assert_eq!(tier.as_str().parse::<CostTier>(), Ok(*tier));
    }
    assert_eq!(CostTier::Low.as_str(), "low");
    assert_eq!(CostTier::Medium.as_str(), "medium");
    assert_eq!(CostTier::High.as_str(), "high");
}

#[test]
fn cost_tier_round_trips_through_json() {
    for tier in CostTier::ALL {
        let encoded = serde_json::to_string(tier).unwrap();
        assert_eq!(encoded, format!("\"{}\"", tier.as_str()));
        assert_eq!(serde_json::from_str::<CostTier>(&encoded).unwrap(), *tier);
    }
}

/// An undeclared tier must not silently read as a real one. `from_stored`
/// reports what it could not read; only then does it fall back.
#[test]
fn cost_tier_reports_an_unknown_value_rather_than_guessing() {
    assert_eq!("free".parse::<CostTier>(),
               Err(UnknownVariant("free".to_string())));

    let (tier, unknown) = CostTier::from_stored("free");
    assert_eq!(tier, CostTier::Medium);
    assert_eq!(unknown, Some(UnknownVariant("free".to_string())));
}

/// The registry ranks on this ordering, so it is part of the contract, not
/// an accident of how the variants happen to be written.
#[test]
fn cost_tier_orders_cheapest_first() {
    assert!(CostTier::Low < CostTier::Medium);
    assert!(CostTier::Medium < CostTier::High);

    let mut tiers = vec![CostTier::High, CostTier::Low, CostTier::Medium];
    tiers.sort();
    assert_eq!(tiers, vec![CostTier::Low, CostTier::Medium, CostTier::High]);
}

/// An agent nobody priced is mid-range, not free and not expensive.
#[test]
fn cost_tier_defaults_to_medium() {
    assert_eq!(CostTier::default(), CostTier::Medium);
}
