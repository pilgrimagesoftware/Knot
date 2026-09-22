//! The signals that drive an agent's row and its inbox: session ids,
//! delivery notices, unread counts, terminal status, and the rules behind
//! the "check your inbox" nudge.

use super::*;

#[test]
fn command_to_send_trims_input_and_rejects_empty_commands() {
    assert_eq!(command_to_send("  cargo test  "), Some("cargo test"));
    assert_eq!(command_to_send("\t\n"), None);
}

#[test]
fn stale_session_ids_excludes_live_agents() {
    let live = Uuid::new_v4();
    let stale = Uuid::new_v4();
    let live_ids = BTreeSet::from([live]);

    assert_eq!(stale_session_ids(&[live, stale], &live_ids), vec![stale]);
}

#[test]
fn delivery_notice_names_the_last_known_recipient_and_counts_events() {
    let mut store = knot_agents::AgentStore::new();
    let first = store.create("~/first", knot_agents::CreateOptions::default());
    let second = store.create("~/second", knot_agents::CreateOptions::default());
    let events = vec![DeliveryEvent { agent_id:   first,
                                      message_id: Uuid::new_v4(), },
                      DeliveryEvent { agent_id:   second,
                                      message_id: Uuid::new_v4(), },
                      DeliveryEvent { agent_id:   second,
                                      message_id: Uuid::new_v4(), },];

    assert_eq!(delivery_notice(&events, store.agents()),
               Some(DeliveryNotice { recipient_name: "second".to_string(),
                                     count:          2, }));
    assert_eq!(delivery_notice(&[], store.agents()), None);
}

#[test]
fn delivery_notice_ignores_unknown_recipients() {
    let store = knot_agents::AgentStore::new();
    let events = [DeliveryEvent { agent_id:   Uuid::new_v4(),
                                  message_id: Uuid::new_v4(), }];

    assert_eq!(delivery_notice(&events, store.agents()), None);
}

#[test]
fn unread_counts_snapshot_includes_zero_and_ignores_other_agents() {
    let mut messages = knot_messaging::MessageStore::new();
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let other = Uuid::new_v4();
    messages.add(knot_messaging::Message::new(other, first, "one"));
    messages.add(knot_messaging::Message::new(other, first, "two"));
    messages.add(knot_messaging::Message::new(other, other, "unrelated"));

    let counts = unread_counts_snapshot(&messages, &[first, second]);

    assert_eq!(counts.get(&first), Some(&2));
    assert_eq!(counts.get(&second), Some(&0));
    assert!(!counts.contains_key(&other));
}

#[test]
fn layout_model_carries_unread_count_into_agent_rows() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let id = store.create("~/alpha", knot_agents::CreateOptions::default());
    let unread_counts = BTreeMap::from([(id, 3)]);

    let model = layout_model(&store, None, &[], &unread_counts);

    assert_eq!(model.selected_agent_rows[0].unread_count, 3);
}

#[test]
fn terminal_status_updates_the_shared_agent_store() {
    let mut store = knot_agents::AgentStore::new();
    let id = store.create("~/alpha", knot_agents::CreateOptions::default());
    let shared = Arc::new(Mutex::new(store));

    apply_terminal_status(&shared, id, knot_agents::AgentState::Running);

    assert_eq!(shared.lock().agent(id).unwrap().state,
               knot_agents::AgentState::Running);
}

/// An idle, MCP-enabled, non-shell agent with an unread message it has not
/// been told about is the only case that nudges.
#[test]
fn inbox_prompt_requires_new_unread_message_for_non_shell_mcp_agent() {
    let message = Uuid::new_v4();
    let ready = NudgeCheck { agent_type:     "claude",
                             mcp_enabled:    true,
                             latest_message: Some(message),
                             last_nudged:    None,
                             idle:           true, };

    assert!(should_inject_inbox_prompt(ready));
    assert!(!should_inject_inbox_prompt(NudgeCheck { last_nudged: Some(message),
                                                     ..ready }),
            "the same message must not nudge twice");
    assert!(!should_inject_inbox_prompt(NudgeCheck { latest_message: None,
                                                     ..ready }));
    assert!(!should_inject_inbox_prompt(NudgeCheck { agent_type: "shell",
                                                     ..ready }),
            "shell agents cannot receive messages");
    assert!(!should_inject_inbox_prompt(NudgeCheck { mcp_enabled: false,
                                                     ..ready }));
    assert!(!should_inject_inbox_prompt(NudgeCheck { idle: false,
                                                     ..ready }),
            "a working agent is nudged when it next goes idle, not now");
}

/// The id carried out is the one the caller must record as nudged, so the
/// predicate and the value can't drift - the caller used to re-derive it
/// with an `expect` after a `true`.
#[test]
fn inbox_prompt_message_id_carries_the_message_the_nudge_is_about() {
    let message = Uuid::new_v4();
    let ready = NudgeCheck { agent_type:     "claude",
                             mcp_enabled:    true,
                             latest_message: Some(message),
                             last_nudged:    None,
                             idle:           true, };

    assert_eq!(inbox_prompt_message_id(ready), Some(message));
    assert_eq!(inbox_prompt_message_id(NudgeCheck { last_nudged: Some(message),
                                                    ..ready }),
               None);
    assert_eq!(inbox_prompt_message_id(NudgeCheck { idle: false,
                                                    ..ready }),
               None);
    assert_eq!(inbox_prompt_message_id(NudgeCheck { latest_message: None,
                                                    ..ready }),
               None);
}

/// The nudge can land behind work the agent had already started, so it has
/// to say what to do when the inbox turns out to be empty. Without the
/// closing instruction the agent reads the prompt as a new task and drops
/// what it was doing - `mcp-messaging`'s "Inbox nudges preserve interrupted
/// session work".
///
/// This asserts the requirement, not the copy: the wording either carries
/// the continuation instruction or the requirement is not met.
#[test]
fn the_inbox_nudge_tells_the_agent_to_continue_its_previous_work() {
    let prompt = app_support::CHECK_INBOX_PROMPT.to_lowercase();

    assert!(prompt.contains("check your inbox"),
            "the nudge must still say to check the inbox");
    assert!(prompt.contains("continue your previous work"),
            "the nudge must tell the agent what to do when there is nothing in the inbox");
}

/// Whether the session can take a prompt this instant is no longer part of
/// the decision. It used to be, and a nudge that failed it was dropped and
/// left to a later poll; delivery queues now, so the decision is only about
/// the message and the agent.
#[test]
fn a_session_busy_right_now_no_longer_suppresses_the_nudge() {
    let message = Uuid::new_v4();
    let check = NudgeCheck { agent_type:     "claude",
                             mcp_enabled:    true,
                             latest_message: Some(message),
                             last_nudged:    None,
                             idle:           true, };

    assert_eq!(inbox_prompt_message_id(check), Some(message));
}

/// A later message re-nudges: the rule is once per message, not once per
/// agent.
#[test]
fn a_second_message_nudges_again() {
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let check = NudgeCheck { agent_type:     "claude",
                             mcp_enabled:    true,
                             latest_message: Some(second),
                             last_nudged:    Some(first),
                             idle:           true, };
    assert!(should_inject_inbox_prompt(check));
}

#[test]
fn awaiting_notice_skips_active_empty_and_duplicate_messages() {
    let agent = Uuid::new_v4();
    assert!(should_show_awaiting_notice(None, agent, "Question?", None));
    assert!(!should_show_awaiting_notice(Some(agent), agent, "Question?", None));
    assert!(!should_show_awaiting_notice(None, agent, "", None));
    assert!(!should_show_awaiting_notice(None, agent, "Question?", Some(&"Question?".to_string())));
}

#[test]
fn agent_status_snapshot_tracks_roster_state_and_registration() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let id = store.create("~/alpha", knot_agents::CreateOptions::default());
    store.create("~/beta", knot_agents::CreateOptions::default());

    let snapshot = agent_status_snapshot(&store);
    assert_eq!(snapshot.len(), 2);
    assert!(snapshot.iter()
                    .find(|key| key.id == id)
                    .unwrap()
                    .status_text
                    .is_empty());

    store.set_state(id, knot_agents::AgentState::Running);
    store.set_status_text(id, "planning".to_string());
    store.set_registered(id, true);
    let updated = agent_status_snapshot(&store);
    assert_ne!(snapshot, updated);
    let key = updated.iter().find(|key| key.id == id).unwrap();
    assert_eq!(key.state, knot_agents::AgentState::Running);
    assert_eq!(key.status_text, "planning");
    assert!(key.is_registered);
    assert_eq!(updated.iter().find(|key| key.id != id).unwrap().state,
               knot_agents::AgentState::Idle);
}
