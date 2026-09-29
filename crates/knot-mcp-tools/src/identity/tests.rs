use knot_agents::CreateOptions;
use knot_mcp::McpSessionManager;
use serde_json::json;

use super::*;

/// Two agents, as the orchestrator and the builder that took its ID were.
fn knot() -> (AgentStore, Uuid, Uuid) {
    let mut store = AgentStore::new();
    let orchestrator = store.create("/tmp/proj", CreateOptions::default());
    let builder = store.create("/tmp/proj", CreateOptions::default());
    (store, orchestrator, builder)
}

fn bound_to(agent: Uuid) -> Caller {
    let sessions = McpSessionManager::new();
    let session = sessions.create_session(agent);
    Caller { agent: Some(agent),
             session_id: Some(session.id),
             sessions }
}

fn is_refused(result: Option<ToolCallResult>) -> bool {
    result.is_some_and(|result| result.is_error == Some(true))
}

#[test]
fn a_connection_acts_as_its_own_agent() {
    let (store, _, builder) = knot();
    let caller = bound_to(builder);
    let args = json!({ "agentId": builder.to_string() });
    assert!(refusal(&store, consts::SET_STATUS, &args, &caller).is_none());
}

/// #539: the builder's connection called with the orchestrator's ID.
#[test]
fn a_connection_naming_another_agent_is_refused_with_its_own_id() {
    let (store, orchestrator, builder) = knot();
    let caller = bound_to(builder);
    for tool in [consts::REGISTER_AGENT,
                 consts::SET_STATUS,
                 consts::CHECK_MESSAGES]
    {
        let args = json!({ "agentId": orchestrator.to_string() });
        let refused = refusal(&store, tool, &args, &caller).expect("refused");
        assert_eq!(refused.is_error, Some(true), "{tool}");
        assert!(refused.content[0].text.contains(&builder.to_string()),
                "{tool}: the refusal says which ID to use");
    }
}

#[test]
fn a_sender_is_held_to_its_connection_too() {
    let (store, orchestrator, builder) = knot();
    let caller = bound_to(builder);
    let args = json!({ "from": orchestrator.to_string(), "to": "x", "content": "hi" });
    assert!(is_refused(refusal(&store, consts::SEND_MESSAGE, &args, &caller)));
}

/// Found in the running app: a connection that arrives before the roster
/// naming its agent was refused its own ID.
#[test]
fn a_connection_is_never_refused_its_own_id() {
    let caller = bound_to(Uuid::new_v4());
    let own = caller.agent.expect("bound").to_string();
    let args = json!({ "agentId": own });
    assert!(refusal(&AgentStore::new(), consts::REGISTER_AGENT, &args, &caller).is_none());
}

#[test]
fn an_unknown_id_on_a_bound_connection_is_told_the_right_one() {
    let (store, _, builder) = knot();
    let caller = bound_to(builder);
    let args = json!({ "agentId": "forgotten" });
    assert!(is_refused(refusal(&store, consts::LIST_AGENTS, &args, &caller)));
}

#[test]
fn an_unbound_connection_may_not_take_a_live_agents_id() {
    let (store, orchestrator, _) = knot();
    let held = bound_to(orchestrator);
    let unbound = Caller { agent:      None,
                           session_id: None,
                           sessions:   held.sessions.clone(), };
    let args = json!({ "agentId": orchestrator.to_string() });
    assert!(is_refused(refusal(&store, consts::REGISTER_AGENT, &args, &unbound)));
}

#[test]
fn an_unbound_connection_may_take_a_free_id() {
    let (store, orchestrator, _) = knot();
    let args = json!({ "agentId": orchestrator.to_string() });
    assert!(refusal(&store, consts::REGISTER_AGENT, &args, &Caller::default()).is_none());
}

#[test]
fn a_call_naming_no_caller_is_left_to_the_tool() {
    let (store, _, builder) = knot();
    assert!(refusal(&store, consts::LIST_REPOS, &json!({}), &bound_to(builder)).is_none());
}

/// Through the catalog, as the server calls it: the orchestrator holds its
/// ID, so a second session registering it is refused where the agent can see
/// it, and a session that registers as itself is then held to that.
#[tokio::test]
async fn a_duplicate_live_registration_is_refused_through_the_catalog() {
    use std::sync::Arc;

    use knot_mcp::ToolCatalog;
    use knot_messaging::NoopNotifier;
    use parking_lot::Mutex;

    let (store, orchestrator, builder) = knot();
    let (_repos, rx) = tokio::sync::watch::channel(Vec::new());
    let catalog =
        crate::McpToolCatalog::new(Arc::new(Mutex::new(store)), rx, Arc::new(NoopNotifier));
    let sessions = McpSessionManager::new();
    let connection = |agent: Option<Uuid>| {
        let session = sessions.create_session(agent.unwrap_or_else(Uuid::nil));
        Caller { agent,
                 session_id: Some(session.id),
                 sessions: sessions.clone() }
    };
    let register = |id: Uuid| json!({ "agentId": id.to_string() });

    let held = connection(Some(orchestrator));
    let ok = catalog.call_as(consts::REGISTER_AGENT, register(orchestrator), &held)
                    .await;
    assert_eq!(ok.is_error, None);

    let unbound = connection(None);
    let refused = catalog.call_as(consts::REGISTER_AGENT, register(orchestrator), &unbound)
                         .await;
    assert_eq!(refused.is_error, Some(true));
    assert!(refused.content[0].text.contains("another live session"));

    let ok = catalog.call_as(consts::REGISTER_AGENT, register(builder), &unbound)
                    .await;
    assert_eq!(ok.is_error, None);
    let session = unbound.session_id.as_deref().expect("a kept session");
    assert_eq!(sessions.session(session).map(|s| s.agent_id),
               Some(builder),
               "registering bound the connection");
    let next_request = Caller { agent: Some(builder),
                                ..unbound.clone() };
    let status = json!({ "agentId": orchestrator.to_string(), "status": "x" });
    let refused = catalog.call_as(consts::SET_STATUS, status, &next_request)
                         .await;
    assert_eq!(refused.is_error, Some(true));
}
