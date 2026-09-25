use tempfile::TempDir;

use super::Logger;
use crate::consts;
use crate::log::entry::{Subject, parse_line};

/// Waits for everything sent so far to reach disk, then closes the channel
/// and waits for the writer to finish. Both steps are barriers rather than
/// waits on a clock, so these tests do not depend on the scheduler.
async fn drain(logger: Logger, task: tokio::task::JoinHandle<()>) {
    logger.flush().await;
    drop(logger);
    task.await.expect("the writer task does not panic");
}

#[tokio::test]
async fn entries_reach_the_file_in_order() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(consts::LOG_FILE_NAME);

    let (logger, task) = Logger::spawn(path.clone());
    for index in 0..25 {
        logger.info(Subject::Request, format!("entry {index}"));
    }
    drain(logger, task).await;

    let contents = std::fs::read_to_string(&path).expect("log file");
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines.len(), 25);
    for (index, line) in lines.iter().enumerate() {
        assert_eq!(parse_line(line)["message"],
                   format!("entry {index}"),
                   "out of order: {line}");
    }
}

#[tokio::test]
async fn logging_does_not_touch_the_filesystem_on_the_calling_task() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(consts::LOG_FILE_NAME);

    let (logger, task) = Logger::spawn(path.clone());
    for index in 0..1000 {
        logger.info(Subject::Request, format!("entry {index}"));
    }

    // This test runs on a current-thread runtime, so the writer task cannot
    // have run yet - nothing has awaited. A thousand sends completing with
    // the file still untouched is what says the send path does no I/O of its
    // own.
    assert!(!path.exists() || std::fs::read_to_string(&path).expect("log file").is_empty(),
            "the request path must not write the log itself");

    drain(logger, task).await;
    assert_eq!(std::fs::read_to_string(&path).expect("log file")
                                             .lines()
                                             .count(),
               1000,
               "and every entry still arrives once the writer runs");
}

#[tokio::test]
async fn a_clone_writes_to_the_same_log() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(consts::LOG_FILE_NAME);

    let (logger, task) = Logger::spawn(path.clone());
    let clone = logger.clone();
    logger.info(Subject::Lifecycle, "from the original");
    clone.warn(Subject::Tool, "from the clone");
    drop(clone);
    drain(logger, task).await;

    let contents = std::fs::read_to_string(&path).expect("log file");
    assert!(contents.contains("from the original"));
    assert!(contents.contains("from the clone"));
}

#[tokio::test]
async fn each_level_reaches_the_file_as_itself() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(consts::LOG_FILE_NAME);

    let (logger, task) = Logger::spawn(path.clone());
    logger.info(Subject::Lifecycle, "ordinary");
    logger.warn(Subject::Lifecycle, "notable");
    logger.error(Subject::Lifecycle, "broken");
    drain(logger, task).await;

    let contents = std::fs::read_to_string(&path).expect("log file");
    let fields: Vec<(String, String, String)> =
        contents.lines()
                .map(parse_line)
                .map(|value| {
                    let field = |key: &str| value[key].as_str().unwrap_or_default().to_owned();
                    (field("level"), field("subject"), field("message"))
                })
                .collect();
    let expected =
        |level: &str, message: &str| (level.to_owned(), "lifecycle".to_owned(), message.to_owned());
    assert_eq!(fields,
               [expected("INFO", "ordinary"),
                expected("WARN", "notable"),
                expected("ERROR", "broken")]);
}

#[tokio::test]
async fn logging_after_the_writer_is_gone_is_silent() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(consts::LOG_FILE_NAME);

    let (logger, task) = Logger::spawn(path);
    task.abort();
    let _ = task.await;

    // Shutdown aborts the writer while handles are still held. A send with
    // nowhere to go must not panic the caller - there is no useful place to
    // report that the log itself has stopped.
    logger.info(Subject::Request, "after shutdown");
}
