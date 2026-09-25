//! The rules that need no window: defaults, parsing, storage and every
//! rejection in `keybindings`' "A customization cannot break other
//! shortcuts".

use knot_core::KeybindingSettings;
use knot_core::ShortcutModifiers;

use super::Chord;
use super::Resolved;
use super::Shortcut;
use super::fixed::reserved_chords;
use super::fixed::taken_chords;
use super::validate;
use super::validate::Rejection;

fn chord(source: &str) -> Chord {
    Chord::parse(source).expect("the test named an unparsable chord")
}

fn command_only() -> ShortcutModifiers {
    ShortcutModifiers { command: true,
                        ..Default::default() }
}

#[test]
fn every_default_parses_and_passes_validation() {
    let defaults = Resolved::defaults();
    for shortcut in Shortcut::ALL {
        assert!(shortcut.default_modifiers().is_some() || shortcut.default_chord().is_some(),
                "{shortcut:?} has no default");
        assert_eq!(validate(&defaults, shortcut),
                   Ok(()),
                   "{shortcut:?}'s default is rejected");
    }
}

#[test]
fn the_defaults_are_the_documented_chords() {
    let defaults = Resolved::defaults();
    let labels = |shortcut| {
        defaults.chords_of(shortcut)
                .iter()
                .map(Chord::label)
                .collect::<Vec<_>>()
    };
    assert_eq!(labels(Shortcut::SelectWorkspace)[2], "⌘3");
    assert_eq!(labels(Shortcut::SelectAgent)[1], "⌥⌘2");
    assert_eq!(labels(Shortcut::SelectAgent).len(), 9);
    assert_eq!(labels(Shortcut::FocusAgentInput), ["⌘L"]);
    assert_eq!(labels(Shortcut::ToggleDashboard), ["⌥⌘O"]);
    assert_eq!(labels(Shortcut::TogglePullRequests), ["⌥⌘P"]);
    assert_eq!(labels(Shortcut::OpenCommandCenter), ["⌥⌘0"]);
    assert_eq!(labels(Shortcut::JumpToBottom), ["⌃⌘↓"]);
}

#[test]
fn a_chord_equals_itself_whatever_order_its_modifiers_were_named_in() {
    assert_eq!(chord("alt-cmd-p"), chord("cmd-alt-p"));
    assert_eq!(chord("cmd-alt-p").to_gpui(), "alt-cmd-p");
    assert_eq!(chord("ctrl-shift-cmd-k").label(), "⌃⇧⌘K");
}

#[test]
fn a_sequence_or_garbage_does_not_parse() {
    assert_eq!(Chord::parse("cmd-k cmd-s"), None);
    assert_eq!(Chord::parse(""), None);
    assert_eq!(Chord::parse("fn-f1"), None);
}

#[test]
fn an_unmodified_chord_is_rejected() {
    let candidate = Resolved::defaults().with_chord(Shortcut::FocusAgentInput, chord("l"));
    assert_eq!(validate(&candidate, Shortcut::FocusAgentInput),
               Err(Rejection::NeedsModifier));
    let shift_only = Resolved::defaults().with_chord(Shortcut::FocusAgentInput, chord("shift-l"));
    assert_eq!(validate(&shift_only, Shortcut::FocusAgentInput),
               Err(Rejection::NeedsModifier));
}

#[test]
fn a_family_modifier_without_a_primary_key_is_rejected() {
    let candidate = Resolved::defaults().with_modifiers(Shortcut::SelectAgent,
                                                        ShortcutModifiers { shift: true,
                                                                            ..Default::default() });
    assert_eq!(validate(&candidate, Shortcut::SelectAgent),
               Err(Rejection::NeedsModifier));
}

#[test]
fn a_fixed_shortcut_cannot_be_taken() {
    let candidate = Resolved::defaults().with_chord(Shortcut::ToggleDashboard, chord("cmd-w"));
    assert_eq!(validate(&candidate, Shortcut::ToggleDashboard),
               Err(Rejection::Conflict { chord:  chord("cmd-w"),
                                         holder: knot_core::l10n::t("keymap.fixed.close_window"), }));
}

#[test]
fn a_text_editing_key_cannot_be_taken() {
    let candidate = Resolved::defaults().with_chord(Shortcut::ToggleDashboard, chord("cmd-c"));
    assert!(matches!(validate(&candidate, Shortcut::ToggleDashboard),
                     Err(Rejection::Conflict { .. })));
}

#[test]
fn the_two_families_cannot_share_a_modifier() {
    let candidate = Resolved::defaults().with_modifiers(Shortcut::SelectAgent, command_only());
    assert_eq!(validate(&candidate, Shortcut::SelectAgent),
               Err(Rejection::Conflict { chord:  chord("cmd-1"),
                                         holder: Shortcut::SelectWorkspace.label(), }));
}

#[test]
fn a_single_chord_cannot_take_a_familys_digit() {
    let candidate = Resolved::defaults().with_chord(Shortcut::FocusAgentInput, chord("cmd-alt-4"));
    assert_eq!(validate(&candidate, Shortcut::FocusAgentInput),
               Err(Rejection::Conflict { chord:  chord("cmd-alt-4"),
                                         holder: Shortcut::SelectAgent.label(), }));
}

#[test]
fn the_workspace_family_cannot_take_the_agent_familys_modifier() {
    // The same collision as above from the other side: the check runs for
    // whichever family changed.
    let candidate = Resolved::defaults().with_modifiers(Shortcut::SelectWorkspace,
                                                        ShortcutModifiers { command: true,
                                                                            alt: true,
                                                                            ..Default::default() });
    assert_eq!(validate(&candidate, Shortcut::SelectWorkspace),
               Err(Rejection::Conflict { chord:  chord("cmd-alt-1"),
                                         holder: Shortcut::SelectAgent.label(), }));
}

#[test]
fn the_rejection_names_the_holder() {
    let rejection = Rejection::Conflict { chord:  chord("cmd-w"),
                                          holder: "Close Window".into(), };
    let message = rejection.message();
    assert!(message.contains("⌘W") && message.contains("Close Window"),
            "{message}");
}

#[test]
fn nothing_is_stored_until_something_is_customized() {
    assert_eq!(Resolved::defaults().to_settings(),
               KeybindingSettings::default());
}

#[test]
fn a_customization_round_trips_through_storage() {
    let customized =
        Resolved::defaults().with_chord(Shortcut::OpenCommandCenter, chord("ctrl-cmd-k"))
                            .with_modifiers(Shortcut::SelectAgent,
                                            ShortcutModifiers { command: true,
                                                                control: true,
                                                                ..Default::default() });
    let stored = customized.to_settings();
    assert_eq!(stored.open_command_center.as_deref(), Some("ctrl-cmd-k"));
    assert_eq!(stored.focus_input, None,
               "an untouched shortcut stays unstored");
    assert_eq!(Resolved::from_settings(&stored), customized);
}

#[test]
fn resetting_restores_the_default() {
    let customized =
        Resolved::defaults().with_chord(Shortcut::ToggleDashboard, chord("ctrl-cmd-b"));
    assert_eq!(customized.reset(Shortcut::ToggleDashboard),
               Resolved::defaults());
}

#[test]
fn a_stored_binding_that_breaks_the_rules_falls_back_to_its_default() {
    let stored = KeybindingSettings { toggle_dashboard: Some("cmd-q".into()),
                                      focus_input: Some("not a chord at all".into()),
                                      agent_select_modifiers: Some(command_only()),
                                      toggle_pull_requests: Some("ctrl-cmd-u".into()),
                                      ..Default::default() };
    let resolved = Resolved::from_settings(&stored);
    assert_eq!(resolved.chord(Shortcut::ToggleDashboard),
               Some(&chord("cmd-alt-o")));
    assert_eq!(resolved.chord(Shortcut::FocusAgentInput),
               Some(&chord("cmd-l")));
    assert_eq!(resolved.modifiers(Shortcut::SelectAgent),
               Shortcut::SelectAgent.default_modifiers());
    assert_eq!(resolved.chord(Shortcut::TogglePullRequests),
               Some(&chord("ctrl-cmd-u")),
               "a valid value beside invalid ones is kept");
}

#[test]
fn two_stored_values_that_collide_keep_the_first() {
    let stored = KeybindingSettings { toggle_dashboard: Some("ctrl-cmd-b".into()),
                                      toggle_pull_requests: Some("ctrl-cmd-b".into()),
                                      ..Default::default() };
    let resolved = Resolved::from_settings(&stored);
    assert_eq!(resolved.chord(Shortcut::ToggleDashboard),
               Some(&chord("ctrl-cmd-b")));
    assert_eq!(resolved.chord(Shortcut::TogglePullRequests),
               Some(&chord("cmd-alt-p")));
}

#[test]
fn every_label_key_resolves() {
    let mut keys: Vec<&str> = Shortcut::ALL.iter()
                                           .map(|shortcut| shortcut.label_key())
                                           .collect();
    keys.extend(taken_chords().iter().map(|(_, key)| *key));
    keys.extend(reserved_chords().iter().map(|(_, key)| *key));
    keys.extend(["keymap.rejection.needs_modifier",
                 "keymap.rejection.conflict"]);
    for key in keys {
        assert_ne!(knot_core::l10n::t(key), key, "{key} does not resolve");
    }
}
