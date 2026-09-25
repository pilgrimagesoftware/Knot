//! A conversation replayed by `session/load`: the user's prompts arrive as
//! user message chunks between the agent's replies, and have to come back
//! as the separate messages they were.

use knot_acp::SessionEvent;
use knot_acp::SessionUpdate;

use crate::panel_state::PanelMessage;
use crate::panel_state::PanelState;

fn user(text: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::UserMessageChunk { text: text.to_string(), })
}

fn agent(text: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::TextDelta { text: text.to_string(), })
}

#[test]
fn a_replayed_conversation_keeps_its_turns_apart() {
    let mut state = PanelState::new();

    for event in [user("register"),
                  agent("registered"),
                  user("fix the build"),
                  agent("fixed")]
    {
        state.apply(event);
    }

    assert_eq!(state.messages,
               vec![PanelMessage::User("register".to_string()),
                    PanelMessage::Assistant("registered".to_string()),
                    PanelMessage::User("fix the build".to_string()),
                    PanelMessage::Assistant("fixed".to_string())]);
}

#[test]
fn a_prompt_replayed_in_chunks_is_one_message() {
    let mut state = PanelState::new();

    state.apply(user("fix "));
    state.apply(user("the build"));

    assert_eq!(state.messages,
               vec![PanelMessage::User("fix the build".to_string())]);
}

/// A replayed prompt was answered long ago, so it must not leave the
/// composer locked behind a turn that will never end.
#[test]
fn a_replayed_prompt_starts_no_turn() {
    let mut state = PanelState::new();

    state.apply(user("fix the build"));

    assert!(!state.turn_active);
}

/// Knot records the prompts it sends itself; an adapter that echoed one
/// back mid-turn must not make it appear twice.
#[test]
fn an_echoed_prompt_during_a_live_turn_is_not_recorded_again() {
    let mut state = PanelState::new();

    state.push_user_message("fix the build".to_string());
    state.apply(user("fix the build"));
    state.apply(agent("fixed"));

    assert_eq!(state.messages,
               vec![PanelMessage::User("fix the build".to_string()),
                    PanelMessage::Assistant("fixed".to_string())]);
}
