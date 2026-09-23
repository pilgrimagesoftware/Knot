//! Unit tests for [`super`].

use std::time::Duration;

use super::*;
use crate::tools::EmptyCatalog;

fn no_agents() -> AgentsSnapshotFn {
    Arc::new(Vec::new)
}

async fn started() -> McpServer {
    let mut server = McpServer::new(0, Arc::new(EmptyCatalog), no_agents());
    server.start()
          .await
          .expect("server starts on an ephemeral port");
    server
}

#[tokio::test]
async fn no_handle_is_lent_before_start_or_after_stop() {
    let mut server = McpServer::new(0, Arc::new(EmptyCatalog), no_agents());
    assert!(server.serve_handle().is_none(),
            "nothing serves before start");

    server.start().await.expect("server starts");
    assert!(server.serve_handle().is_some(),
            "a started server lends its handle");

    server.stop();
    assert!(server.serve_handle().is_none(),
            "a stopped server lends nothing");
}

#[tokio::test]
async fn the_lent_handle_stays_pending_while_the_server_serves() {
    let mut server = started().await;
    let handle = server.serve_handle().expect("started");

    let outcome = tokio::time::timeout(Duration::from_millis(100), handle).await;

    assert!(outcome.is_err(),
            "a serving task must not look like one that ended");
    server.stop();
}

#[tokio::test]
async fn the_lent_handle_completes_with_an_error_when_the_serve_task_is_aborted() {
    let mut server = started().await;
    let handle = server.serve_handle().expect("started");
    handle.abort();

    let joined = tokio::time::timeout(Duration::from_secs(5), server.serve_handle()
                                                                   .expect("still held"))
                     .await
                     .expect("an aborted task completes promptly");

    let error = joined.expect_err("an aborted task does not complete successfully");
    assert!(error.is_cancelled(),
            "the abort must be reported as a cancellation: {error:?}");
}
