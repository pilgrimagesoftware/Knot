//! Reading the markdown section's file off the render path: the read, the
//! note shown when it fails, and the watch that re-reads it.
//!
//! The document tests run a real runtime and a real file, because what they
//! pin is that a result arrives from another thread and is flagged - the
//! hand-off `repaint_poll_tick` depends on.

use std::time::Duration;
use std::time::Instant;

use tempfile::TempDir;

use super::MarkdownDocument;
use super::read_markdown;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread().enable_all()
                                               .build()
                                               .expect("a test runtime")
}

/// Polls `document` until a read lands, as the repaint poll would.
fn wait_for_landing(document: &MarkdownDocument, within: Duration) -> bool {
    let deadline = Instant::now() + within;
    while Instant::now() < deadline {
        if document.take_landed() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn a_readable_file_reads_as_its_contents() {
    let dir = TempDir::new().expect("a temporary directory");
    let path = dir.path().join("plan.md");
    std::fs::write(&path, "# Plan\n").expect("the file is written");

    assert_eq!(read_markdown(&path), "# Plan\n");
}

/// Shown where the file would be, so it has to name what went wrong and
/// where - not resolve to its own catalogue key.
#[test]
fn a_missing_file_reads_as_a_note_naming_it() {
    let dir = TempDir::new().expect("a temporary directory");
    let path = dir.path().join("gone.md");

    let note = read_markdown(&path);

    assert_ne!(note, "artifact_panel.read_failed");
    assert!(note.contains(&path.display().to_string()), "{note}");
}

#[test]
fn opening_a_document_reads_it_off_the_calling_thread_and_flags_it() {
    let dir = TempDir::new().expect("a temporary directory");
    let path = dir.path().join("plan.md");
    std::fs::write(&path, "first").expect("the file is written");
    let runtime = runtime();

    let document = MarkdownDocument::open(path, &runtime);

    assert!(wait_for_landing(&document, Duration::from_secs(5)),
            "the first read lands and says so");
    assert_eq!(document.body().as_deref(), Some("first"));
    assert!(!document.take_landed(), "the flag clears as it is read");
}

/// What re-reading per frame used to give for free: an agent editing the
/// file it showed has the edit reach the screen.
#[test]
fn an_edit_to_the_file_is_read_again() {
    let dir = TempDir::new().expect("a temporary directory");
    let path = dir.path().join("plan.md");
    std::fs::write(&path, "first").expect("the file is written");
    let runtime = runtime();
    let document = MarkdownDocument::open(path.clone(), &runtime);
    assert!(wait_for_landing(&document, Duration::from_secs(5)));

    // Past the watch's debounce and any coalescing the platform's event
    // source does, so the write is not folded into the first read's events.
    std::thread::sleep(Duration::from_millis(500));
    std::fs::write(&path, "second").expect("the file is rewritten");

    assert!(wait_for_landing(&document, Duration::from_secs(10)),
            "the watch schedules a second read");
    assert_eq!(document.body().as_deref(), Some("second"));
}
