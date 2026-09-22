//! Desktop notifications: the decision layer behind
//! `openspec/specs/desktop-notifications/spec.md`.
//!
//! One scenario cannot be covered here. `UNUserNotificationCenter` aborts
//! outside an app bundle, so delivery itself is only exercisable from
//! `Knot.app` (`make package`) - these prove *when* a notification is
//! raised and what it says, not that macOS displayed it.

use gpui_kit::SystemNotificationResponse;
use uuid::Uuid;

use crate::app_state::AWAITING_INPUT_DEFAULT_BODY;
use crate::app_state::notification_body;
use crate::app_state::notification_response_agent_id;
use crate::app_state::should_notify;
use crate::app_state::should_show_awaiting_notice;

#[test]
fn should_notify_requires_setting_and_notice() {
    assert!(should_notify(true, true));
    assert!(!should_notify(false, true));
    assert!(!should_notify(true, false));
    assert!(!should_notify(false, false));
}

#[test]
fn notification_body_uses_message_when_present() {
    assert_eq!(notification_body("Grant access?"), "Grant access?");
}

#[test]
fn notification_body_defaults_on_empty() {
    assert_eq!(notification_body(""), AWAITING_INPUT_DEFAULT_BODY);
}

#[test]
fn notification_response_agent_id_parses_valid_tag() {
    let id = Uuid::new_v4();
    let response = SystemNotificationResponse { tag:       id.to_string().into(),
                                                action_id: None, };
    assert_eq!(notification_response_agent_id(&response), Some(id));
}

#[test]
fn notification_response_agent_id_none_for_invalid_tag() {
    let response = SystemNotificationResponse { tag:       "not-a-uuid".into(),
                                                action_id: None, };
    assert_eq!(notification_response_agent_id(&response), None);
}

/// The suppression the spec names first: an agent the user is already
/// looking at does not need announcing.
#[test]
fn the_visible_agent_is_not_announced() {
    let agent = Uuid::new_v4();
    assert!(!should_show_awaiting_notice(Some(agent), agent, "Grant access?", None));
    assert!(should_show_awaiting_notice(Some(Uuid::new_v4()), agent, "Grant access?", None));
    assert!(should_show_awaiting_notice(None, agent, "Grant access?", None));
}

/// `Effect::AwaitingInput` fires on every status event reporting Input, not
/// only on the transition, so the same prompt arrives repeatedly while the
/// user has not answered it. Only the first is news.
#[test]
fn a_second_event_for_the_same_prompt_is_a_repeat() {
    let agent = Uuid::new_v4();
    let prompt = "Grant filesystem access?".to_string();

    assert!(should_show_awaiting_notice(None, agent, &prompt, None));
    assert!(!should_show_awaiting_notice(None, agent, &prompt, Some(&prompt)));
    assert!(should_show_awaiting_notice(None, agent, "Something else?", Some(&prompt)));
}

/// The spec's "default body without a message" scenario. The ported
/// predicate returned `false` for an absent message, so this had never been
/// met - an agent blocked without hook text stayed silent, which is the case
/// most worth surfacing.
#[test]
fn an_agent_with_no_message_still_raises_one_with_the_default_body() {
    let agent = Uuid::new_v4();

    assert!(should_show_awaiting_notice(None, agent, "", None));
    assert_eq!(notification_body(""), "Needs your attention");
}

/// An empty message repeats like any other: the agent is still sitting on
/// the same unanswered prompt.
#[test]
fn a_repeated_empty_message_is_still_a_repeat() {
    let agent = Uuid::new_v4();
    let empty = String::new();

    assert!(should_show_awaiting_notice(None, agent, "", None));
    assert!(!should_show_awaiting_notice(None, agent, "", Some(&empty)));
}

/// The tag is the agent id, which is both how a click routes back and how a
/// newer notification replaces a still-showing one for the same agent.
#[test]
fn the_tag_round_trips_as_the_agent_id() {
    let agent = Uuid::new_v4();
    let response = SystemNotificationResponse { tag:       agent.to_string().into(),
                                                action_id: None, };

    assert_eq!(notification_response_agent_id(&response), Some(agent));
}
