use time::OffsetDateTime;
use time::macros::datetime;

use super::{Entry, Level, Subject, escape};

fn at(message: &str) -> Entry {
    Entry { at:      datetime!(2026-09-22 15:04:05.123456 UTC),
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
fn a_line_is_timestamp_then_level_then_subject_then_message() {
    let line = at("initialize").render();

    let mut fields = line.splitn(4, ' ');
    assert_eq!(fields.next(), Some("2026-09-22T15:04:05.123456Z"));
    assert_eq!(fields.next(), Some("INFO"));
    assert_eq!(fields.next(), Some("request"));
    assert_eq!(fields.next(), Some("initialize"));
}

#[test]
fn the_timestamp_carries_a_sub_second_component() {
    let line = at("initialize").render();
    let timestamp = line.split(' ')
                        .next()
                        .expect("a line always has a first field");

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
    let line = at("read /tmp/odd\nname failed").render();

    assert_eq!(line.lines().count(), 1, "one entry is one line: {line}");
    assert!(line.ends_with("read /tmp/odd\\nname failed"));
}

#[test]
fn a_carriage_return_does_not_break_the_line() {
    let line = at("progress\r100%").render();

    assert_eq!(line.lines().count(), 1, "one entry is one line: {line}");
    assert!(line.ends_with("progress\\r100%"));
}

#[test]
fn a_backslash_is_escaped_so_the_escaping_is_reversible() {
    // Without this, a message ending in a backslash followed by an `n` is
    // indistinguishable from one containing a newline.
    assert_eq!(escape(r"C:\notes"), r"C:\\notes");
    assert_eq!(escape("a\nb"), "a\\nb");
}

#[test]
fn an_ordinary_message_is_left_alone() {
    assert_eq!(escape("tools/call send-message"), "tools/call send-message");
}
