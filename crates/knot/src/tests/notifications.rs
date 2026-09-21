//! Desktop notifications.
//!
//! NOTE: these cover the decision layer only. Nothing in the app calls
//! `gpui::App::show_system_notification`, so
//! `openspec/specs/desktop-notifications/spec.md` is not actually
//! implemented - these tests passing does not mean a notification is ever
//! raised. See the issue tracking that gap.

use super::*;

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
