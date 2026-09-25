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

/// One line of the file, parsed. Parsing every line is itself an assertion:
/// the file is JSON Lines, so a line that is not an object is a defect.
#[derive(Debug)]
struct Logged {
    level:   String,
    subject: String,
    message: String,
}

fn entries(path: &Path) -> Vec<Logged> {
    read(path).lines()
              .map(|line| {
                  let value: Value = serde_json::from_str(line)
                      .unwrap_or_else(|error| panic!("not a JSON line ({error}): {line}"));
                  let field = |key: &str| value[key].as_str().unwrap_or_default().to_owned();
                  Logged { level:   field("level"),
                           subject: field("subject"),
                           message: field("message"), }
              })
              .collect()
}

/// The first entry about `subject` whose message starts with `prefix`.
fn find<'a>(entries: &'a [Logged], subject: &str, prefix: &str) -> Option<&'a Logged> {
    entries.iter()
           .find(|entry| entry.subject == subject && entry.message.starts_with(prefix))
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

    let logged = entries(&path);
    assert!(find(&logged, "lifecycle", "binding").is_some(),
            "the attempt is logged: {logged:?}");
    assert!(find(&logged, "lifecycle", &format!("bound {bound}")).is_some(),
            "with the address it actually bound: {logged:?}");
    assert!(find(&logged, "lifecycle", "stopped").is_some(),
            "and so is the stop: {logged:?}");
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

    let logged = entries(&path);
    let failure = logged.iter()
                        .find(|entry| entry.subject == "lifecycle" && entry.level == "ERROR")
                        .unwrap_or_else(|| panic!("a failed bind is an error: {logged:?}"));
    assert!(failure.message.starts_with("bind"), "{failure:?}");
    assert!(failure.message.contains(&format!("127.0.0.1:{taken}")),
            "naming the address it could not have: {failure:?}");
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

    let logged = entries(&path);
    let request = find(&logged, "request", "[").unwrap_or_else(|| panic!("no request: {logged:?}"))
                                               .message
                                               .as_str();
    let response =
        find(&logged, "response", "[").unwrap_or_else(|| panic!("no response: {logged:?}"))
                                      .message
                                      .as_str();

    assert!(request.contains("initialize"));
    assert!(response.contains("initialize"));
    assert!(response.ends_with(" ok"),
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

    let logged = entries(&path);
    let response =
        find(&logged, "response", "[").unwrap_or_else(|| panic!("no response: {logged:?}"));
    assert!(response.message.ends_with("nonexistent error"),
            "an unknown method's response is recorded as an error: {response:?}");
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

    assert!(find(&entries(&path), "tool", "tools/list -> 1 tools").is_some());
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
