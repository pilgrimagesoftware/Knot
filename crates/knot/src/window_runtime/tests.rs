//! Teardown with a blocking task that will not finish on its own.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::WindowRuntime;
use crate::consts::WINDOW_RUNTIME_SHUTDOWN_TIMEOUT;

/// A plain `Runtime` would wait for this task forever - it blocks until the
/// test lets it go, which is after the drop. The window's runtime gives up
/// after its bound.
#[test]
fn dropping_does_not_wait_for_a_blocking_task_that_never_ends() {
    let runtime = WindowRuntime::new().expect("a runtime");
    let (release, released) = mpsc::channel::<()>();
    let (started_tx, started_rx) = mpsc::channel();
    runtime.spawn_blocking(move || {
               let _ = started_tx.send(());
               let _ = released.recv();
           });
    started_rx.recv_timeout(Duration::from_secs(10))
              .expect("the blocking task starts");

    let dropping = Instant::now();
    drop(runtime);
    let took = dropping.elapsed();

    // Let the detached task end, so the test leaves no thread behind.
    let _ = release.send(());
    assert!(took < WINDOW_RUNTIME_SHUTDOWN_TIMEOUT + Duration::from_secs(2),
            "dropping the window's runtime took {took:?}");
}

/// The bound is a ceiling, not a delay: with nothing running, teardown is
/// immediate.
#[test]
fn dropping_an_idle_runtime_is_immediate() {
    let runtime = WindowRuntime::new().expect("a runtime");

    let dropping = Instant::now();
    drop(runtime);

    assert!(dropping.elapsed() < WINDOW_RUNTIME_SHUTDOWN_TIMEOUT,
            "an idle runtime took {:?} to drop",
            dropping.elapsed());
}
