use knot_agents::AgentStore;
use knot_mcp::ToolCallResult;
use knot_messaging::{DeliveryNotifier, MessageStore, broadcast, check, send};

use crate::ActivationQueue;
use crate::args::{optional_bool, require_str};
use crate::lookup::{agent_not_found, find_in_workspace, workspace_members};
use crate::responses::{
    BroadcastResponse, CheckMessagesResponse, MessageInfo, SendMessageResponse, success,
};

/// `activation` is where a recipient that was not activated is recorded for
/// the app to start, per `mcp-messaging`'s "Direct send activates a
/// deactivated recipient". `None` outside the app (tests, benches): the
/// message still sends, nothing starts it.
pub fn send_message(agents: &AgentStore, messages: &mut MessageStore,
                    notifier: &dyn DeliveryNotifier, activation: Option<&ActivationQueue>,
                    arguments: &serde_json::Value)
                    -> ToolCallResult {
    let from = match require_str(arguments, "from") {
        Ok(v) => v,
        Err(err) => return err,
    };
    let to = match require_str(arguments, "to") {
        Ok(v) => v,
        Err(err) => return err,
    };
    let content = match require_str(arguments, "content") {
        Ok(v) => v,
        Err(err) => return err,
    };

    let Some(sender) = crate::lookup::find_by_name_or_id(agents, from)
    else {
        return agent_not_found(agents, from);
    };
    let sender = sender.clone();
    let members = workspace_members(agents, sender.id);

    let Some(recipient) = find_in_workspace(agents, sender.id, to)
    else {
        return ToolCallResult::error("Failed to send message: Recipient not found");
    };

    let recipient_id = recipient.id;
    let recipient_activated = recipient.activated;

    match send(messages, notifier, &sender, &members, recipient_id, content) {
        Ok(_) => {
            // Only once the routing rules above have passed - a rejected
            // send activates nobody. The store flag, not session presence,
            // is what `agent-lifecycle` states the rule in terms of.
            if !recipient_activated && let Some(queue) = activation {
                queue.lock().push(recipient_id);
            }
            success(&SendMessageResponse {
            success: true,
            message: "Message sent successfully. Don't check for a response right away - you will be notified when the other agent responds.".to_string(),
        })
        }
        Err(err) => ToolCallResult::error(err.to_string()),
    }
}

pub fn check_messages(agents: &AgentStore, messages: &mut MessageStore,
                      arguments: &serde_json::Value)
                      -> ToolCallResult {
    let agent_id_str = match require_str(arguments, "agentId") {
        Ok(v) => v,
        Err(err) => return err,
    };
    let Some(agent) = crate::lookup::find_by_name_or_id(agents, agent_id_str)
    else {
        return agent_not_found(agents, agent_id_str);
    };
    let mark_as_read = optional_bool(arguments, "markAsRead").unwrap_or(true);

    let unread = check(messages, agent.id, mark_as_read);
    let infos = unread.into_iter()
                      .map(|m| {
                          let sender_name = agents.agent(m.from)
                                                  .map(|a| a.name.clone())
                                                  .unwrap_or_else(|| m.from.to_string());
                          MessageInfo { id:        m.id.to_string(),
                                        from:      sender_name,
                                        content:   m.content,
                                        timestamp: rfc3339(m.timestamp), }
                      })
                      .collect();

    success(&CheckMessagesResponse { messages: infos })
}

pub fn broadcast_message(agents: &AgentStore, messages: &mut MessageStore,
                         notifier: &dyn DeliveryNotifier, arguments: &serde_json::Value)
                         -> ToolCallResult {
    let from = match require_str(arguments, "from") {
        Ok(v) => v,
        Err(err) => return err,
    };
    let content = match require_str(arguments, "content") {
        Ok(v) => v,
        Err(err) => return err,
    };

    let Some(sender) = crate::lookup::find_by_name_or_id(agents, from)
    else {
        return agent_not_found(agents, from);
    };
    let sender = sender.clone();
    let members = workspace_members(agents, sender.id);

    let count = broadcast(messages, notifier, &sender, &members, content);
    success(&BroadcastResponse { success:         count > 0,
                                 recipient_count: count, })
}

/// RFC 3339, matching the Swift reference's `ISO8601DateFormatter`.
fn rfc3339(time: std::time::SystemTime) -> String {
    time::OffsetDateTime::from(time).format(&time::format_description::well_known::Rfc3339)
                                    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use knot_agents::CreateOptions;
    use knot_messaging::NoopNotifier;
    use serde_json::json;

    use super::*;

    fn registered(store: &mut AgentStore, folder: &str) -> uuid::Uuid {
        let id = store.create(folder, CreateOptions::default());
        store.set_registered(id, true);
        id
    }

    fn activation_queue() -> ActivationQueue {
        std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()))
    }

    #[test]
    fn send_message_succeeds_between_registered_workspace_members() {
        let mut agents = AgentStore::new();
        let from = registered(&mut agents, "/tmp/a");
        let to = registered(&mut agents, "/tmp/b");
        let mut messages = MessageStore::new();

        let result = send_message(&agents,
                                  &mut messages,
                                  &NoopNotifier,
                                  None,
                                  &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

        assert_eq!(result.is_error, None);
        assert!(messages.has_unread(to));
    }

    #[test]
    fn send_message_to_shell_agent_surfaces_rejection_text() {
        let mut agents = AgentStore::new();
        let from = registered(&mut agents, "/tmp/a");
        let to = agents.create("/tmp/shell",
                               CreateOptions { agent_type: Some("shell".to_string()),
                                               insert_after: Some(from),
                                               ..Default::default() });
        agents.set_registered(to, true);
        let mut messages = MessageStore::new();

        let result = send_message(&agents,
                                  &mut messages,
                                  &NoopNotifier,
                                  None,
                                  &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

        assert_eq!(result.is_error, Some(true));
        assert_eq!(result.content[0].text,
                   "Cannot send messages to shell agents");
    }

    #[test]
    fn check_messages_marks_read_by_default() {
        let mut agents = AgentStore::new();
        let from = registered(&mut agents, "/tmp/a");
        let to = registered(&mut agents, "/tmp/b");
        let mut messages = MessageStore::new();
        send_message(&agents,
                     &mut messages,
                     &NoopNotifier,
                     None,
                     &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

        let result = check_messages(&agents, &mut messages, &json!({"agentId": to.to_string()}));

        assert_eq!(result.is_error, None);
        assert!(!messages.has_unread(to));
    }

    #[test]
    fn check_messages_without_mark_as_read_leaves_unread() {
        let mut agents = AgentStore::new();
        let from = registered(&mut agents, "/tmp/a");
        let to = registered(&mut agents, "/tmp/b");
        let mut messages = MessageStore::new();
        send_message(&agents,
                     &mut messages,
                     &NoopNotifier,
                     None,
                     &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

        check_messages(&agents,
                       &mut messages,
                       &json!({"agentId": to.to_string(), "markAsRead": false}));

        assert!(messages.has_unread(to));
    }

    #[test]
    fn direct_send_queues_a_deactivated_recipient_for_activation() {
        let mut agents = AgentStore::new();
        let from = registered(&mut agents, "/tmp/a");
        let to = registered(&mut agents, "/tmp/b");
        // `create` leaves an agent deactivated until something starts it,
        // which is the state this queues on.
        assert!(!agents.agent(to).unwrap().activated);
        let mut messages = MessageStore::new();
        let activation = activation_queue();

        let result = send_message(&agents,
                                  &mut messages,
                                  &NoopNotifier,
                                  Some(&activation),
                                  &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

        assert_eq!(result.is_error, None);
        assert_eq!(&*activation.lock(), &[to]);
    }

    #[test]
    fn direct_send_to_an_activated_recipient_queues_nothing() {
        let mut agents = AgentStore::new();
        let from = registered(&mut agents, "/tmp/a");
        let to = registered(&mut agents, "/tmp/b");
        agents.set_activated(to, true);
        let mut messages = MessageStore::new();
        let activation = activation_queue();

        let result = send_message(&agents,
                                  &mut messages,
                                  &NoopNotifier,
                                  Some(&activation),
                                  &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

        assert_eq!(result.is_error, None);
        assert!(activation.lock().is_empty());
    }

    #[test]
    fn a_send_rejected_by_a_routing_rule_queues_nothing() {
        let mut agents = AgentStore::new();
        let from = registered(&mut agents, "/tmp/a");
        // Shell recipient: rejected by `send`'s shell-agent rule, after the
        // recipient resolves - the case where a queue push would be easiest
        // to misplace.
        let shell = agents.create("/tmp/shell",
                                  CreateOptions { agent_type: Some("shell".to_string()),
                                                  insert_after: Some(from),
                                                  ..Default::default() });
        agents.set_registered(shell, true);
        let mut messages = MessageStore::new();
        let activation = activation_queue();

        let result = send_message(&agents,
                                  &mut messages,
                                  &NoopNotifier,
                                  Some(&activation),
                                  &json!({"from": from.to_string(), "to": shell.to_string(), "content": "hi"}));

        assert_eq!(result.is_error, Some(true));
        assert!(activation.lock().is_empty());
    }

    #[test]
    fn a_send_to_an_unresolvable_recipient_queues_nothing() {
        let mut agents = AgentStore::new();
        let from = registered(&mut agents, "/tmp/a");
        let mut messages = MessageStore::new();
        let activation = activation_queue();

        let result = send_message(&agents,
                                  &mut messages,
                                  &NoopNotifier,
                                  Some(&activation),
                                  &json!({"from": from.to_string(), "to": uuid::Uuid::new_v4().to_string(), "content": "hi"}));

        assert_eq!(result.is_error, Some(true));
        assert!(activation.lock().is_empty());
    }

    #[test]
    fn broadcast_never_queues_a_deactivated_recipient() {
        let mut agents = AgentStore::new();
        let sender = registered(&mut agents, "/tmp/a");
        let deactivated = registered(&mut agents, "/tmp/b");
        assert!(!agents.agent(deactivated).unwrap().activated);
        let mut messages = MessageStore::new();
        let activation = activation_queue();

        let result = broadcast_message(&agents,
                                       &mut messages,
                                       &NoopNotifier,
                                       &json!({"from": sender.to_string(), "content": "hi all"}));

        assert_eq!(result.is_error, None);
        assert!(messages.has_unread(deactivated));
        // `broadcast_message` takes no queue at all - the strongest form of
        // "a broadcast activates nobody" this layer can assert.
        assert!(activation.lock().is_empty());
    }

    #[test]
    fn broadcast_reports_recipient_count() {
        let mut agents = AgentStore::new();
        let sender = registered(&mut agents, "/tmp/a");
        registered(&mut agents, "/tmp/b");
        registered(&mut agents, "/tmp/c");
        let mut messages = MessageStore::new();

        let result = broadcast_message(&agents,
                                       &mut messages,
                                       &NoopNotifier,
                                       &json!({"from": sender.to_string(), "content": "hi all"}));

        assert_eq!(result.is_error, None);
        assert!(result.content[0].text.contains("\"recipientCount\": 2"));
    }
}
