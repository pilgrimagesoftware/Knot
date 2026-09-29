use std::path::PathBuf;

use super::*;

fn attachment(kind: LogKind, tail: Option<&str>) -> Attachment {
    Attachment { kind,
                 path: Some(PathBuf::from("/tmp/Knot/knot.log")),
                 tail: tail.map(str::to_owned) }
}

#[test]
fn the_tail_keeps_the_last_lines_in_order() {
    assert_eq!(tail(b"one\ntwo\nthree\nfour\n", 2), "three\nfour");
}

#[test]
fn a_short_log_is_kept_whole() {
    assert_eq!(tail(b"one\ntwo", 200), "one\ntwo");
}

#[test]
fn a_stray_byte_does_not_cost_the_log() {
    let text = tail(b"ok\n\xFFbad\nfine\n", 10);
    assert!(text.starts_with("ok\n") && text.ends_with("fine"), "{text}");
}

#[test]
fn a_log_is_attached_collapsed_and_fenced() {
    let body = attachments_markdown(&[attachment(LogKind::Mcp, Some("line ``` with fence"))]);
    assert!(body.starts_with("<details>\n<summary>"), "{body}");
    assert!(body.contains("````\nline ``` with fence\n````"), "{body}");
    assert!(body.contains("</details>"), "{body}");
}

#[test]
fn an_unreadable_log_is_named_rather_than_dropped() {
    let body = attachments_markdown(&[attachment(LogKind::App, None)]);
    assert!(body.contains("/tmp/Knot/knot.log"), "{body}");
    assert!(!body.contains("<details>"), "{body}");
}

#[test]
fn the_browser_fallback_names_the_files() {
    let body = attachments_by_path(&[attachment(LogKind::App, Some("secret"))]);
    assert!(body.contains("/tmp/Knot/knot.log"), "{body}");
    assert!(!body.contains("secret"),
            "the fallback URL must not carry the log");
}

#[test]
fn every_log_label_resolves() {
    for kind in LogKind::ALL {
        assert!(!kind.label().starts_with("bug_report."), "{kind:?}");
    }
    for key in ["bug_report.logs.unreadable",
                "bug_report.logs.attach_by_hand",
                "bug_report.logs.no_directory",
                "bug_report.logs.hint"]
    {
        assert!(!knot_core::l10n::t(key).starts_with("bug_report."), "{key}");
    }
}

#[test]
fn the_tail_of_a_large_file_starts_on_a_whole_line() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("big.log");
    let line = "x".repeat(99);
    let content = (0..1000).map(|i| format!("{i:04}{line}\n"))
                           .collect::<String>();
    std::fs::write(&path, &content).expect("write");

    let bytes = read_tail(&path).expect("read");
    assert!(bytes.len() <= crate::consts::BUG_REPORT_LOG_TAIL_BYTES);
    let text = String::from_utf8(bytes).expect("utf8");
    assert!(text.lines().all(|l| l.len() == 103), "a fragment was kept");
    assert!(text.ends_with(&format!("0999{line}\n")));
}
