use serde_json::Value;
use time::OffsetDateTime;
use time::macros::datetime;

use super::{Entry, Level, Subject};

/// The rendered line, parsed back - which is itself the first assertion:
/// every line is a JSON object.
fn parsed(entry: &Entry) -> Value {
    let line = entry.render();
    serde_json::from_str(&line).unwrap_or_else(|error| panic!("not JSON ({error}): {line}"))
}

fn at(message: &str) -> Entry {
    Entry { at:      datetime!(2026-09-22 15:04:05.123456 UTC),
            pid:     4242,
            level:   Level::Info,
            subject: Subject::Request,
            message: message.to_string(), }
}

#[test]
fn every_level_renders_its_own_word() {
    assert_eq!(Level::Info.to_string(), "INFO");
    assert_eq!(Level::Warn.to_string(), "WARN");
    assert_eq!(Level::Error.to_string(), "ERROR");
}

#[test]
fn every_subject_renders_its_own_word() {
    assert_eq!(Subject::Lifecycle.to_string(), "lifecycle");
    assert_eq!(Subject::Request.to_string(), "request");
    assert_eq!(Subject::Response.to_string(), "response");
    assert_eq!(Subject::Tool.to_string(), "tool");
    assert_eq!(Subject::Heartbeat.to_string(), "heartbeat");
    assert_eq!(Subject::Log.to_string(), "log");
}

#[test]
fn no_two_subjects_share_a_word() {
    let all = [Subject::Lifecycle,
               Subject::Request,
               Subject::Response,
               Subject::Tool,
               Subject::Heartbeat,
               Subject::Log];
    let mut words: Vec<String> = all.iter().map(ToString::to_string).collect();
    words.sort();
    let count = words.len();
    words.dedup();
    assert_eq!(words.len(),
               count,
               "a shared word makes the log unfilterable by subject");
}

#[test]
fn a_line_is_time_then_pid_then_level_then_subject_then_message() {
    let line = at("initialize").render();

    assert_eq!(line,
               r#"{"time":"2026-09-22T15:04:05.123456Z","pid":4242,"level":"INFO","subject":"request","message":"initialize"}"#);
}

/// Every instance sharing a home directory writes the same file; the ID is
/// what tells their lines apart.
#[test]
fn a_new_entry_carries_this_process_id() {
    let entry = Entry::now(Level::Info, Subject::Lifecycle, "stopped");

    assert_eq!(entry.pid, std::process::id());
    assert_eq!(parsed(&entry)["pid"], std::process::id());
}

#[test]
fn the_timestamp_carries_a_sub_second_component() {
    let value = parsed(&at("initialize"));
    let timestamp = value["time"].as_str().expect("time is a string");

    assert!(timestamp.contains('.'),
            "a sub-second component is what orders two entries within the same second: \
             {timestamp}");
    assert!(timestamp.ends_with('Z'),
            "the timestamp is UTC: {timestamp}");
}

#[test]
fn now_stamps_the_moment_the_event_happened() {
    let before = OffsetDateTime::now_utc();
    let entry = Entry::now(Level::Warn, Subject::Lifecycle, "bind failed");
    let after = OffsetDateTime::now_utc();

    assert!(entry.at >= before && entry.at <= after);
    assert_eq!(entry.level, Level::Warn);
    assert_eq!(entry.subject, Subject::Lifecycle);
    assert_eq!(entry.message, "bind failed");
}

#[test]
fn an_embedded_newline_does_not_break_the_line() {
    let entry = at("read /tmp/odd\nname failed");
    let line = entry.render();

    assert_eq!(line.lines().count(), 1, "one entry is one line: {line}");
    assert_eq!(parsed(&entry)["message"], "read /tmp/odd\nname failed");
}

#[test]
fn a_carriage_return_does_not_break_the_line() {
    let entry = at("progress\r100%");
    let line = entry.render();

    assert_eq!(line.lines().count(), 1, "one entry is one line: {line}");
    assert_eq!(parsed(&entry)["message"], "progress\r100%");
}

/// A message is recovered exactly, however it is spelled - JSON's escaping
/// is reversible where a hand-rolled one has to be proven so.
#[test]
fn a_message_round_trips_unchanged() {
    for message in [r"C:\notes",
                    "a\\nb",
                    "quote \" inside",
                    "tools/call send-message"]
    {
        assert_eq!(parsed(&at(message))["message"], message);
    }
}
