//! The writer's own tests. Every one drives [`Writer`] directly rather than
//! through [`crate::log::Logger`]: the channel adds scheduling to assertions
//! that are about bytes on disk.

use std::path::Path;

use tempfile::TempDir;

use super::{Writer, rolled};
use crate::consts;
use crate::log::entry::parse_line;
use crate::log::entry::{Entry, Level, Subject};

fn entry(message: &str) -> Entry {
    Entry::now(Level::Info, Subject::Lifecycle, message)
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn a_missing_directory_is_created() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join("nested/deeper/knot-mcp.jsonl");

    let mut writer = Writer::open(path.clone());
    writer.write(&entry("bound 127.0.0.1:8767"));

    assert!(path.exists(), "the directory and file are created on open");
    assert!(read(&path).contains("bound 127.0.0.1:8767"));
}

#[test]
fn an_existing_file_is_appended_to_not_truncated() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(consts::LOG_FILE_NAME);

    let mut first = Writer::open(path.clone());
    first.write(&entry("first run"));
    drop(first);

    let mut second = Writer::open(path.clone());
    second.write(&entry("second run"));

    let contents = read(&path);
    assert!(contents.contains("first run"),
            "a restart must not discard the record of what preceded it");
    assert!(contents.contains("second run"));
}

#[test]
fn the_byte_counter_is_seeded_from_an_existing_file() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(consts::LOG_FILE_NAME);
    std::fs::write(&path, "x".repeat(990)).expect("seed file");

    // Cap 1000, 990 already there: the very first entry crosses it. Without
    // the seed the writer would think the file was empty and overshoot by
    // most of a cap.
    let mut writer = Writer::open_with(path.clone(), 1000, 3);
    writer.write(&entry("crosses the cap"));

    assert!(read(&rolled(&path, 1)).starts_with("xxx"),
            "the pre-existing content was rolled aside");
    assert!(read(&path).contains("crosses the cap"));
}

#[test]
fn reaching_the_cap_rolls_the_file() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(consts::LOG_FILE_NAME);

    let mut writer = Writer::open_with(path.clone(), 200, 3);
    for index in 0..10 {
        writer.write(&entry(&format!("entry {index}")));
    }

    assert!(rolled(&path, 1).exists(),
            "the active file was rolled aside");
    assert!(read(&path).len() <= 200,
            "the active file stays under the cap");
}

#[test]
fn rolled_files_beyond_the_retained_count_are_deleted() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(consts::LOG_FILE_NAME);

    let mut writer = Writer::open_with(path.clone(), 120, 2);
    for index in 0..40 {
        writer.write(&entry(&format!("entry {index}")));
    }

    assert!(rolled(&path, 1).exists());
    assert!(rolled(&path, 2).exists());
    assert!(!rolled(&path, 3).exists(),
            "a retained count of two keeps .1 and .2 and no more");
}

#[test]
fn rotation_loses_nothing_and_interleaves_nothing() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(consts::LOG_FILE_NAME);
    let total = 60;
    // Around ten entries per file, so the run rolls a handful of times -
    // and a retained count far above that, because this test is about
    // whether a roll splits or drops an entry, not about the deletion of
    // old files that `rolled_files_beyond_the_retained_count_are_deleted`
    // covers.
    let retained = 20;

    let mut writer = Writer::open_with(path.clone(), 600, retained);
    for index in 0..total {
        writer.write(&entry(&format!("entry {index}")));
    }

    // Oldest file first, so reassembling them yields the original order.
    let mut lines: Vec<String> = Vec::new();
    for index in (1..=retained).rev() {
        lines.extend(read(&rolled(&path, index)).lines().map(str::to_string));
    }
    lines.extend(read(&path).lines().map(str::to_string));

    assert_eq!(lines.len(),
               total,
               "every entry appears exactly once across the files");
    for (index, line) in lines.iter().enumerate() {
        assert_eq!(parse_line(line)["message"],
                   format!("entry {index}"),
                   "entry {index} is out of order or split across a roll: {line}");
    }
}

#[test]
fn an_unopenable_path_still_accepts_entries() {
    let root = TempDir::new().expect("temp dir");
    // A file where the writer expects a directory: `create_dir_all` fails,
    // so the log never opens.
    let blocker = root.path().join("blocked");
    std::fs::write(&blocker, "not a directory").expect("blocker");
    let path = blocker.join(consts::LOG_FILE_NAME);

    let mut writer = Writer::open(path);
    writer.write(&entry("served anyway"));

    assert!(writer.degraded, "the failure was noticed");
    assert!(writer.file.is_none(), "and left no file behind");
}

#[test]
fn a_failure_is_reported_once_per_episode() {
    let root = TempDir::new().expect("temp dir");
    let blocker = root.path().join("blocked");
    std::fs::write(&blocker, "not a directory").expect("blocker");

    let mut writer = Writer::open(blocker.join(consts::LOG_FILE_NAME));
    assert!(writer.degraded, "opening reported the first failure");

    // Every later write finds no file and must stay silent. The flag
    // staying set across all of them is what says nothing was printed
    // again: `report` is the only thing that prints, and it is a no-op
    // while the flag is set.
    for _ in 0..5 {
        writer.write(&entry("still failing"));
        assert!(writer.degraded);
    }
}

#[test]
fn recovery_ends_the_episode_silently() {
    let root = TempDir::new().expect("temp dir");
    let path = root.path().join(consts::LOG_FILE_NAME);

    let mut writer = Writer::open(path.clone());
    writer.degraded = true;

    writer.write(&entry("working again"));

    assert!(!writer.degraded, "a successful write ends the episode");
    assert!(read(&path).contains("working again"));
}

#[test]
fn a_rolled_path_is_the_active_path_with_an_index() {
    let path = Path::new("/tmp/knot/knot-mcp.jsonl");

    assert_eq!(rolled(path, 1), Path::new("/tmp/knot/knot-mcp.jsonl.1"));
    assert_eq!(rolled(path, 3), Path::new("/tmp/knot/knot-mcp.jsonl.3"));
}
