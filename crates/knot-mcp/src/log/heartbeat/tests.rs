use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tempfile::TempDir;
use uuid::Uuid;

use super::{Vitals, line, spawn};
use crate::consts;
use crate::log::logger::Logger;
use crate::session::McpSessionManager;

fn vitals(log: Logger, interval: Duration) -> Vitals {
    Vitals { log,
             addr: "127.0.0.1:8767".parse().expect("a literal address"),
             started: Instant::now(),
             sessions: McpSessionManager::new(),
             served: Arc::new(AtomicU64::new(0)),
             interval }
}

#[tokio::test]
async fn a_heartbeat_carries_every_vital() {
    let root = TempDir::new().expect("temp dir");
    let (log, _task) = Logger::spawn(root.path().join(consts::LOG_FILE_NAME));
    let vitals = vitals(log, consts::HEARTBEAT_INTERVAL);
    vitals.sessions.create_session(Uuid::new_v4());
    vitals.sessions.create_session(Uuid::new_v4());
    vitals.served.store(7, Ordering::Relaxed);

    let text = line(&vitals);

    assert!(text.contains("up "), "uptime: {text}");
    assert!(text.contains("127.0.0.1:8767"), "bound address: {text}");
    assert!(text.contains("2 sessions"), "live session count: {text}");
    assert!(text.contains("7 requests"),
            "requests this interval: {text}");
}

#[tokio::test]
async fn an_idle_interval_reports_zero_rather_than_nothing() {
    let root = TempDir::new().expect("temp dir");
    let (log, _task) = Logger::spawn(root.path().join(consts::LOG_FILE_NAME));
    let vitals = vitals(log, consts::HEARTBEAT_INTERVAL);

    let text = line(&vitals);

    assert!(text.contains("0 requests"),
            "an idle server is exactly what the heartbeat exists to distinguish from a dead \
             one: {text}");
    assert!(text.contains("0 sessions"));
}

#[tokio::test]
async fn each_heartbeat_describes_its_own_interval() {
    let root = TempDir::new().expect("temp dir");
    let (log, _task) = Logger::spawn(root.path().join(consts::LOG_FILE_NAME));
    let vitals = vitals(log, consts::HEARTBEAT_INTERVAL);

    vitals.served.store(5, Ordering::Relaxed);
    let first = line(&vitals);
    vitals.served.store(2, Ordering::Relaxed);
    let second = line(&vitals);
    let third = line(&vitals);

    assert!(first.contains("5 requests"), "{first}");
    assert!(second.contains("2 requests"),
            "two, not seven: the first interval's count must not still be in this one: {second}");
    assert!(third.contains("0 requests"),
            "and the count resets when taken: {third}");
}

/// The clock is paused, so no part of this waits on wall time. Each round
/// advances a full interval and then drains the log through the writer's own
/// barrier; how many rounds it takes for the task to be polled is the
/// scheduler's business, which is why the first assertion counts at least
/// one rather than exactly three.
#[tokio::test(start_paused = true)]
async fn heartbeats_are_written_while_running_and_stop_with_the_task() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(consts::LOG_FILE_NAME);
    let (log, writer) = Logger::spawn(path.clone());
    let interval = Duration::from_secs(60);
    let beats = |text: &str| text.lines().filter(|l| l.contains("heartbeat")).count();
    let count = || beats(&std::fs::read_to_string(&path).expect("log file"));

    let task = spawn(vitals(log.clone(), interval));
    for _ in 0..5 {
        tokio::time::advance(interval).await;
        log.flush().await;
    }
    let while_running = count();
    assert!(while_running > 0,
            "a running heartbeat writes: {while_running} entries");

    // Awaiting the aborted task is what makes the rest of this exact:
    // afterwards the task is provably gone, so anything it had queued is
    // already in the channel and the flush below puts all of it on disk.
    task.abort();
    let _ = task.await;
    log.flush().await;
    let at_stop = count();

    for _ in 0..5 {
        tokio::time::advance(interval).await;
        log.flush().await;
    }
    assert_eq!(count(),
               at_stop,
               "a stopped heartbeat writes nothing further");

    drop(log);
    let _ = writer.await;
}
