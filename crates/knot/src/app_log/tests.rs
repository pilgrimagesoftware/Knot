use super::*;

#[test]
fn a_new_run_keeps_the_last_runs_log_and_starts_empty() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join(APP_LOG_PREVIOUS_FILE_NAME), "two runs ago").expect("write");
    std::fs::write(dir.path().join(APP_LOG_FILE_NAME), "last run").expect("write");

    drop(open_fresh_log(dir.path()).expect("open"));

    let read = |name| std::fs::read_to_string(dir.path().join(name)).expect("read");
    assert_eq!(read(APP_LOG_FILE_NAME), "");
    assert_eq!(read(APP_LOG_PREVIOUS_FILE_NAME), "last run");
}

#[test]
fn the_first_run_makes_the_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let logs = dir.path().join("Logs/Knot");

    drop(open_fresh_log(&logs).expect("open"));

    assert!(logs.join(APP_LOG_FILE_NAME).exists());
    assert!(!logs.join(APP_LOG_PREVIOUS_FILE_NAME).exists());
}
