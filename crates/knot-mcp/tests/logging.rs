//! What the running server writes to its log file.
//!
//! Every test here drives a real `McpServer` over HTTP against a log in a
//! temporary directory, because the thing under test is what reaches disk
//! after a request has gone through the whole stack - not what any one
//! function returns.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use knot_agents::Agent;
use knot_mcp::{
    EmptyCatalog, LOG_FILE_NAME, McpServer, ToolCallResult, ToolCatalog, ToolDefinition,
    ToolInputSchema,
};
use serde_json::{Value, json};
use tempfile::TempDir;

fn no_agents() -> Arc<dyn Fn() -> Vec<Agent> + Send + Sync> {
    Arc::new(Vec::new)
}

struct EchoCatalog;

#[async_trait]
impl ToolCatalog for EchoCatalog {
    fn list(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition { name:         "send-message".to_string(),
                              description:  "Sends a message".to_string(),
                              input_schema: ToolInputSchema::default(), }]
    }

    async fn call(&self, _name: &str, _arguments: Value) -> ToolCallResult {
        ToolCallResult::ok("sent")
    }
}

async fn start(catalog: Arc<dyn ToolCatalog>) -> (McpServer, String, TempDir, PathBuf) {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join("logs").join(LOG_FILE_NAME);
    let mut server = McpServer::new(0, catalog, no_agents()).with_log(path.clone());
    server.start().await.expect("server starts");
    let addr = server.bound_addr().expect("bound address");
    (server, format!("http://{addr}"), root, path)
}

/// Stops the server and returns once everything it logged - the stop entry
/// included - is on disk.
///
/// The handle is cloned before stopping so the writer still has a live
/// sender to answer the barrier through, and so that this waits on the log
/// rather than on the scheduler. No sleeping, no yielding a guessed number
/// of times: the assertions that follow see a finished file every run.
async fn settle(server: &mut McpServer) {
    let log = server.log().cloned();
    server.stop();
    if let Some(log) = log {
        log.flush().await;
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

#[tokio::test]
async fn the_log_file_and_its_directory_are_created() {
    let (mut server, _base, _root, path) = start(Arc::new(EmptyCatalog)).await;
    settle(&mut server).await;

    assert!(path.exists(),
            "the log directory was created along with the file");
}

#[tokio::test]
async fn binding_and_stopping_are_logged() {
    let (mut server, _base, _root, path) = start(Arc::new(EmptyCatalog)).await;
    let bound = server.bound_addr().expect("bound address");
    settle(&mut server).await;

    let contents = read(&path);
    assert!(contents.contains("lifecycle binding"),
            "the attempt is logged: {contents}");
    assert!(contents.contains(&format!("lifecycle bound {bound}")),
            "with the address it actually bound: {contents}");
    assert!(contents.contains("lifecycle stopped"),
            "and so is the stop: {contents}");
}

#[tokio::test]
async fn a_bind_failure_is_logged_with_its_error() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(LOG_FILE_NAME);

    // Hold a port, then ask a second server for the same one.
    let (mut holder, _base, _holder_root, _holder_path) = start(Arc::new(EmptyCatalog)).await;
    let taken = holder.bound_addr().expect("bound address").port();

    let mut blocked =
        McpServer::new(taken, Arc::new(EmptyCatalog), no_agents()).with_log(path.clone());
    let result = blocked.start().await;
    assert!(result.is_err(), "the port is already held");
    settle(&mut blocked).await;
    settle(&mut holder).await;

    let contents = read(&path);
    assert!(contents.contains("ERROR lifecycle bind"),
            "a failed bind is an error, not an info: {contents}");
    assert!(contents.contains(&format!("127.0.0.1:{taken}")),
            "naming the address it could not have: {contents}");
}

#[tokio::test]
async fn a_method_round_trip_is_logged_both_ways() {
    let (mut server, base, _root, path) = start(Arc::new(EmptyCatalog)).await;

    reqwest::Client::new().post(format!("{base}/mcp"))
                          .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }))
                          .send()
                          .await
                          .unwrap();
    settle(&mut server).await;

    let contents = read(&path);
    let request = contents.lines()
                          .find(|line| line.contains("request "))
                          .unwrap_or_else(|| panic!("no request entry in: {contents}"));
    let response = contents.lines()
                           .find(|line| line.contains("response "))
                           .unwrap_or_else(|| panic!("no response entry in: {contents}"));

    assert!(request.contains("initialize"));
    assert!(response.contains("initialize"));
    assert!(response.contains(" ok"),
            "the response records that it was not an error");

    // Both carry the same session, which is what lets a reader pair them up
    // in a file holding several clients' traffic.
    let session = |line: &str| {
        line.split('[')
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .map(str::to_string)
            .expect("an entry names its session")
    };
    assert_eq!(session(request), session(response));
}

#[tokio::test]
async fn an_errored_response_is_logged_as_an_error_outcome() {
    let (mut server, base, _root, path) = start(Arc::new(EmptyCatalog)).await;

    reqwest::Client::new().post(format!("{base}/mcp"))
                          .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "nonexistent" }))
                          .send()
                          .await
                          .unwrap();
    settle(&mut server).await;

    let contents = read(&path);
    assert!(contents.contains("response [") && contents.contains("nonexistent error"),
            "an unknown method's response is recorded as an error: {contents}");
}

#[tokio::test]
async fn the_tool_catalog_size_is_logged() {
    let (mut server, base, _root, path) = start(Arc::new(EchoCatalog)).await;

    reqwest::Client::new().post(format!("{base}/mcp"))
                          .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }))
                          .send()
                          .await
                          .unwrap();
    settle(&mut server).await;

    assert!(read(&path).contains("tool tools/list -> 1 tools"));
}

#[tokio::test]
async fn an_unwritable_log_path_does_not_stop_the_server() {
    let root = TempDir::new().expect("temp dir");
    // A file where the log wants a directory, so nothing can be created
    // under it. This is the packaged-app case where the Logs directory is
    // unavailable: the server must not care.
    let blocker = root.path().join("blocked");
    std::fs::write(&blocker, "not a directory").expect("blocker");

    let mut server =
        McpServer::new(0, Arc::new(EchoCatalog), no_agents()).with_log(blocker.join(LOG_FILE_NAME));
    server.start()
          .await
          .expect("the server starts with no usable log");
    let base = format!("http://{}", server.bound_addr().expect("bound address"));

    let response = reqwest::Client::new().post(format!("{base}/mcp"))
                                         .json(&json!({
                                                   "jsonrpc": "2.0",
                                                   "id": 1,
                                                   "method": "tools/list",
                                               }))
                                         .send()
                                         .await
                                         .expect("the request is served");
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.expect("a JSON body");
    assert_eq!(body["result"]["tools"][0]["name"], "send-message",
               "requests are served with the same results as when logging works");

    settle(&mut server).await;
    assert!(!blocker.join(LOG_FILE_NAME).exists(),
            "and no log was conjured up");
}

#[tokio::test]
async fn a_tool_call_records_its_shape_and_not_its_arguments() {
    let (mut server, base, _root, path) = start(Arc::new(EchoCatalog)).await;
    let secret = "the whole prompt the user typed, and /Users/someone/secret.txt";

    reqwest::Client::new().post(format!("{base}/mcp"))
                          .json(&json!({
                                    "jsonrpc": "2.0",
                                    "id": 1,
                                    "method": "tools/call",
                                    "params": {
                                        "name": "send-message",
                                        "arguments": { "agentId": "a1", "message": secret },
                                    },
                                }))
                          .send()
                          .await
                          .unwrap();
    settle(&mut server).await;

    let contents = read(&path);
    assert!(contents.contains("tools/call send-message"),
            "the call is logged: {contents}");
    assert!(contents.contains("keys=[agentId, message]"),
            "with its shape: {contents}");
    assert!(contents.contains("bytes="),
            "and its payload size: {contents}");

    assert!(!contents.contains(secret),
            "the argument values must not reach a durable file: {contents}");
    assert!(!contents.contains("/Users/someone/secret.txt"));
}
