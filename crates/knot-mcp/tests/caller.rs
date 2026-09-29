//! The connection a tool call arrives on reaches the catalog as a `Caller`
//! bound to the agent its URL names (#539).

use std::sync::Arc;

use async_trait::async_trait;
use knot_agents::Agent;
use knot_mcp::{Caller, McpServer, ToolCallResult, ToolCatalog, ToolDefinition};
use parking_lot::Mutex;
use serde_json::{Value, json};
use uuid::Uuid;

/// Answers every call with the agent its caller is bound to, or `none`.
struct WhoCalls;

#[async_trait]
impl ToolCatalog for WhoCalls {
    fn list(&self) -> Vec<ToolDefinition> {
        Vec::new()
    }

    async fn call(&self, _name: &str, _arguments: Value) -> ToolCallResult {
        ToolCallResult::ok("unreached")
    }

    async fn call_as(&self, _name: &str, _arguments: Value, caller: &Caller) -> ToolCallResult {
        ToolCallResult::ok(caller.agent
                                 .map_or_else(|| "none".to_owned(), |id| id.to_string()))
    }
}

async fn start() -> (McpServer, String) {
    let no_agents: Arc<dyn Fn() -> Vec<Agent> + Send + Sync> = Arc::new(Vec::new);
    let mut server = McpServer::new(0, Arc::new(WhoCalls), no_agents);
    server.start().await.expect("server starts");
    let addr = server.bound_addr().expect("bound address");
    (server, format!("http://{addr}/mcp"))
}

/// Initializes at `url`, then makes one tool call on the session it got,
/// and returns who the catalog saw.
async fn caller_seen(url: &str) -> String {
    let client = reqwest::Client::new();
    let init = client.post(url)
                     .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }))
                     .send()
                     .await
                     .unwrap();
    let session = init.headers()["Mcp-Session-Id"].to_str()
                                                  .unwrap()
                                                  .to_owned();
    let call = client.post(url)
                     .header("Mcp-Session-Id", session)
                     .json(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                                    "params": { "name": "any", "arguments": {} } }))
                     .send()
                     .await
                     .unwrap();
    let body: Value = call.json().await.unwrap();
    body["result"]["content"][0]["text"].as_str()
                                        .unwrap()
                                        .to_owned()
}

#[tokio::test]
async fn a_url_naming_an_agent_binds_its_connection() {
    let (mut server, url) = start().await;
    let agent = Uuid::new_v4();
    assert_eq!(caller_seen(&format!("{url}?agent={agent}")).await,
               agent.to_string());
    server.stop();
}

#[tokio::test]
async fn a_url_naming_no_agent_leaves_the_connection_unbound() {
    let (mut server, url) = start().await;
    assert_eq!(caller_seen(&url).await, "none");
    assert_eq!(caller_seen(&format!("{url}?agent=not-a-uuid")).await,
               "none");
    server.stop();
}

/// Records every agent the server reports connected.
#[derive(Default)]
struct Connections(Mutex<Vec<Uuid>>);

#[async_trait]
impl ToolCatalog for Connections {
    fn list(&self) -> Vec<ToolDefinition> {
        Vec::new()
    }

    async fn call(&self, _name: &str, _arguments: Value) -> ToolCallResult {
        ToolCallResult::ok("")
    }

    fn connected(&self, agent_id: Uuid) {
        self.0.lock().push(agent_id);
    }
}

/// #552: the server reports a bound connection as soon as it initializes,
/// before any tool call - a resumed agent that never calls one is still
/// registered.
#[tokio::test]
async fn a_bound_connection_is_reported_on_initialize() {
    let catalog = Arc::new(Connections::default());
    let no_agents: Arc<dyn Fn() -> Vec<Agent> + Send + Sync> = Arc::new(Vec::new);
    let mut server = McpServer::new(0, catalog.clone(), no_agents);
    server.start().await.expect("server starts");
    let url = format!("http://{}/mcp", server.bound_addr().expect("bound address"));
    let agent = Uuid::new_v4();
    let client = reqwest::Client::new();
    for target in [format!("{url}?agent={agent}"), url.clone()] {
        client.post(&target)
              .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }))
              .send()
              .await
              .unwrap();
    }
    assert_eq!(*catalog.0.lock(),
               vec![agent],
               "an unbound connection reports nobody");
    server.stop();
}
