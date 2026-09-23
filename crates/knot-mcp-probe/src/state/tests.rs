use super::{ALL_STATES, ServerState};

#[test]
fn every_state_round_trips_through_its_token() {
    for state in ALL_STATES {
        let parsed: ServerState = state.token().parse().expect("its own token parses");
        assert_eq!(parsed, *state);
        assert_eq!(state.to_string(), state.token());
    }
}

/// The roster is what the other tests iterate, so a variant missing from it
/// would make every one of them pass while covering less. Six is the number
/// the real tooling reports; changing it is a spec change.
#[test]
fn the_roster_holds_every_variant() {
    assert_eq!(ALL_STATES.len(), 6);

    let mut tokens: Vec<_> = ALL_STATES.iter().map(|s| s.token()).collect();
    tokens.sort_unstable();
    tokens.dedup();
    assert_eq!(tokens.len(), ALL_STATES.len(), "two states share a token");
}

#[test]
fn a_token_naming_no_state_is_rejected() {
    assert!("broken".parse::<ServerState>().is_err());
    assert!("".parse::<ServerState>().is_err());
    assert!("Connected".parse::<ServerState>().is_err(),
            "the token is the lowercase form, not the variant name");
}

/// The distinction the change exists to preserve. A server the user disabled
/// is not a server that needs attention, and counting it as one would put a
/// repair prompt on a deliberate choice.
#[test]
fn only_unwanted_states_need_attention() {
    assert!(ServerState::NeedsAuthentication.needs_attention());
    assert!(ServerState::Failed.needs_attention());

    assert!(!ServerState::Disabled.needs_attention());
    assert!(!ServerState::PendingApproval.needs_attention());
    assert!(!ServerState::Connected.needs_attention());
    assert!(!ServerState::Unknown.needs_attention());
}

/// Wider than needing attention: approving and re-enabling are both things
/// the agent's own flow does, so those rows still offer the handover.
#[test]
fn the_action_is_offered_wherever_the_agents_flow_helps() {
    assert!(ServerState::NeedsAuthentication.offers_action());
    assert!(ServerState::Failed.offers_action());
    assert!(ServerState::PendingApproval.offers_action());
    assert!(ServerState::Disabled.offers_action());

    assert!(!ServerState::Connected.offers_action(),
            "there is no remedy to hand over for a server that works");
    assert!(!ServerState::Unknown.offers_action(),
            "nothing is known to help a state we could not classify");
}

/// Needing attention implies there is something to do about it. A state that
/// was one without the other would leave the header naming a row whose
/// action is absent.
#[test]
fn anything_needing_attention_offers_the_action() {
    for state in ALL_STATES {
        if state.needs_attention() {
            assert!(state.offers_action(),
                    "{state} needs attention but offers no action");
        }
    }
}
