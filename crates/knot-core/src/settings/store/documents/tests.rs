use std::fs;

use serde::Deserialize;
use tempfile::tempdir;

use super::{read_collection, read_object, write, write_collection};
use crate::consts::DOCUMENT_TEMP_EXTENSION;

#[derive(Debug, PartialEq, Deserialize)]
struct Record {
    name: String,
}

#[test]
fn a_written_document_leaves_no_temporary_behind() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("personas.json");

    write(&path, "[]").unwrap();

    assert_eq!(fs::read_to_string(&path).unwrap(), "[]");
    assert!(!path.with_extension(DOCUMENT_TEMP_EXTENSION).exists());
}

#[test]
fn writing_creates_the_parent_directory() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nested").join("personas.json");

    write(&path, "[]").unwrap();

    assert!(path.exists());
}

#[test]
fn a_missing_document_reads_as_an_empty_collection() {
    let dir = tempdir().unwrap();

    let records: Vec<Record> = read_collection(&dir.path().join("absent.json"));

    assert!(records.is_empty());
}

#[test]
fn an_unreadable_document_reads_as_an_empty_collection() {
    let dir = tempdir().unwrap();
    // A directory where a file is expected: `read` fails rather than
    // returning bytes, which is the unreadable case the contract names.
    let path = dir.path().join("personas.json");
    fs::create_dir(&path).unwrap();

    let records: Vec<Record> = read_collection(&path);

    assert!(records.is_empty());
}

#[test]
fn a_non_json_document_reads_as_an_empty_collection() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("personas.json");
    fs::write(&path, "not json at all").unwrap();

    let records: Vec<Record> = read_collection(&path);

    assert!(records.is_empty());
}

#[test]
fn a_non_array_document_reads_as_an_empty_collection() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("personas.json");
    fs::write(&path, r#"{"name":"an object, not an array"}"#).unwrap();

    let records: Vec<Record> = read_collection(&path);

    assert!(records.is_empty());
}

#[test]
fn a_bad_record_is_dropped_and_its_neighbours_survive() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("personas.json");
    fs::write(&path, r#"[{"name":"first"},{"nome":42},{"name":"last"}]"#).unwrap();

    let records: Vec<Record> = read_collection(&path);

    assert_eq!(records,
               vec![Record { name: "first".to_string(), },
                    Record { name: "last".to_string(), }]);
}

#[test]
fn a_collection_round_trips_through_a_bare_array() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("recent-repos.json");

    write_collection(&path, &["alpha", "beta"]).unwrap();

    assert!(fs::read_to_string(&path).unwrap()
                                     .trim_start()
                                     .starts_with('['));
    let records: Vec<String> = read_collection(&path);
    assert_eq!(records, vec!["alpha".to_string(), "beta".to_string()]);
}

#[test]
fn read_object_rejects_anything_that_is_not_an_object() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("absent.json");
    let array = dir.path().join("array.json");
    let garbage = dir.path().join("garbage.json");
    let object = dir.path().join("preferences.json");
    fs::write(&array, "[1, 2]").unwrap();
    fs::write(&garbage, "}{").unwrap();
    fs::write(&object, r#"{"mcpServerPort":9000}"#).unwrap();

    assert!(read_object(&missing).is_none());
    assert!(read_object(&array).is_none());
    assert!(read_object(&garbage).is_none());
    assert!(read_object(&object).is_some());
}
