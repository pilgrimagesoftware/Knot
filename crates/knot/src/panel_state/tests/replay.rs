//! A conversation replayed by `session/load`: the user's prompts arrive as
//! user message chunks between the agent's replies, and have to come back
//! as the separate messages they were.

use knot_acp::SessionEvent;
use knot_acp::SessionUpdate;

use crate::panel_state::PanelMessage;
use crate::panel_state::PanelState;

fn user(text: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::UserMessageChunk { text: text.to_string(),
                                                           meta: None, })
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

fn injected(text: &str, kind: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::UserMessageChunk { text: text.to_string(),
                                                           meta: Some(serde_json::json!({ "_claude/origin": { "kind": kind } })), })
}

/// The #551 case: a background task finished between two turns, and the
/// replay shows it as a note, not as a prompt the user typed.
#[test]
fn a_replayed_task_notification_is_a_notice_not_a_prompt() {
    let mut state = PanelState::new();

    for event in [user("run the tests"),
                  agent("running them in the background"),
                  user("<task-notification><summary>Tests passed</summary></task-notification>"),
                  agent("all green")]
    {
        state.apply(event);
    }

    let notice = knot_core::l10n::t_with("panel.harness.task_finished",
                                         &[("summary", "Tests passed")]);
    assert_eq!(state.messages,
               vec![PanelMessage::User("run the tests".to_string()),
                    PanelMessage::Assistant("running them in the background".to_string()),
                    PanelMessage::Notice(notice),
                    PanelMessage::Assistant("all green".to_string())]);
}

/// Claude Code appends reminders to a prompt as their own block, which
/// replays as its own chunk: the prompt stays whole and the reminder goes.
#[test]
fn a_reminder_replayed_after_a_prompt_does_not_join_it() {
    let mut state = PanelState::new();

    state.apply(user("fix the build"));
    state.apply(user("<system-reminder>Be brief.</system-reminder>"));

    assert_eq!(state.messages,
               vec![PanelMessage::User("fix the build".to_string())]);
}

#[test]
fn an_origin_tagged_chunk_is_not_a_prompt() {
    let mut state = PanelState::new();

    state.apply(injected("<task-notification><summary>Done</summary></task-notification>",
                         "task-notification"));

    assert!(!state.messages
                  .iter()
                  .any(|message| matches!(message, PanelMessage::User(_))),
            "{:?}",
            state.messages);
}

#[test]
fn a_prompt_mentioning_a_harness_tag_is_still_a_prompt() {
    let mut state = PanelState::new();

    state.apply(user("what is a <task-notification>?"));

    assert_eq!(state.messages,
               vec![PanelMessage::User("what is a <task-notification>?".to_string())]);
}
