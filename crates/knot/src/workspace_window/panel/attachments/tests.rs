//! The reference text an attachment occupies in the buffer.
//!
//! The reconciliation that uses it needs a window and a composer, so it is
//! `tests::panel_chips`; what is checkable here is the text itself, which
//! is the thing both sides have to agree on.

use std::path::Path;
use std::path::PathBuf;

use crate::composer_scan::Construct;
use crate::composer_scan::scan;
use crate::workspace_window::panel::attachments::attachment_reference;
use crate::workspace_window::panel::attachments::insertion_text;
use crate::workspace_window::panel::attachments::surviving_attachments;

#[test]
fn a_reference_is_the_files_own_name() {
    assert_eq!(attachment_reference(Path::new("/tmp/shots/screenshot.png")),
               "@screenshot.png",
               "a chip is read at a glance; the full path still reaches the agent on the \
                `Attached:` line");
}

#[test]
fn a_name_with_a_space_is_escaped() {
    assert_eq!(attachment_reference(Path::new("/tmp/my notes.md")),
               r"@my\ notes.md");
}

/// A path with no file name at all still produces something to stand for
/// it, rather than an empty token that could never be found again.
#[test]
fn a_path_without_a_name_falls_back_to_the_path() {
    let reference = attachment_reference(&PathBuf::from("/"));

    assert!(reference.starts_with('@'));
    assert!(reference.len() > 1);
}

/// The reference has to read back as one attachment run, or the chip is
/// drawn across part of itself and the reconciliation cannot find it.
#[test]
fn a_reference_scans_as_one_attachment() {
    for path in ["/tmp/plain.rs", "/tmp/my notes.md", "/tmp/two  spaces.md"] {
        let reference = attachment_reference(Path::new(path));
        let buffer = format!("look at {reference} please");

        let chips: Vec<&str> =
            scan(&buffer, &[reference.as_str()]).into_iter()
                                                .filter(|span| {
                                                    span.construct == Construct::Attachment
                                                })
                                                .map(|span| &buffer[span.range])
                                                .collect();

        assert_eq!(chips,
                   vec![reference.as_str()],
                   "{path:?} did not scan as one chip");
    }
}

/// Two files with the same name produce the same reference, which is why
/// the reconciliation counts occurrences instead of merely looking for
/// one.
#[test]
fn two_files_with_one_name_share_a_reference() {
    let first = attachment_reference(Path::new("/a/notes.md"));
    let second = attachment_reference(Path::new("/b/notes.md"));

    assert_eq!(first, second,
               "the reconciliation has to survive this, and it does so by counting");
}

/// The paths, as the pending-context table holds them.
fn table(paths: &[&str]) -> Vec<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

/// A buffer holding a chip for each of `paths`.
fn buffer_with(paths: &[&str]) -> String {
    let chips: Vec<String> = paths.iter()
                                  .map(|path| attachment_reference(Path::new(path)))
                                  .collect();
    format!("look at {}", chips.join(" "))
}

/// `acp-panel-ui`: "Deleting the reference removes the strip entry."
#[test]
fn a_chip_the_user_deleted_detaches_its_row() {
    let paths = table(&["/tmp/a.rs", "/tmp/b.rs"]);
    let buffer = buffer_with(&["/tmp/a.rs"]);

    let kept = surviving_attachments(&buffer, &paths, &[]);

    assert_eq!(kept,
               table(&["/tmp/a.rs"]),
               "the row whose chip is gone is the row that detaches");
}

#[test]
fn a_chip_still_present_keeps_its_row() {
    let paths = table(&["/tmp/a.rs"]);
    let buffer = buffer_with(&["/tmp/a.rs"]);

    assert_eq!(surviving_attachments(&buffer, &paths, &[]), paths);
}

#[test]
fn an_emptied_buffer_detaches_everything() {
    let paths = table(&["/tmp/a.rs", "/tmp/b.rs"]);

    assert!(surviving_attachments("", &paths, &[]).is_empty());
}

/// Two files with the same name share a reference, so deleting one chip
/// has to detach exactly one row - the case that makes this count
/// occurrences rather than look for the text.
#[test]
fn two_files_with_one_name_detach_one_at_a_time() {
    let paths = table(&["/a/notes.md", "/b/notes.md"]);
    let both = format!("{} {}",
                       attachment_reference(Path::new("/a/notes.md")),
                       attachment_reference(Path::new("/b/notes.md")));

    assert_eq!(surviving_attachments(&both, &paths, &[]).len(),
               2,
               "two chips, two rows");

    let one = attachment_reference(Path::new("/a/notes.md"));
    assert_eq!(surviving_attachments(&one, &paths, &[]).len(),
               1,
               "one chip left, so one row - not both, and not none");
}

/// A row whose chip has not been written yet is not a row the user
/// deleted. Without this, attaching would detach itself in the window
/// between the table row and the deferred insertion.
#[test]
fn a_row_waiting_for_its_chip_is_not_detached() {
    let paths = table(&["/tmp/just-attached.png"]);
    let queued = paths.clone();

    assert_eq!(surviving_attachments("", &paths, &queued),
               paths,
               "its chip is legitimately not in the buffer yet");
}

/// `panel-rich-input`: "Attaching with an empty buffer" - the reference is
/// the buffer's only content, so no separator is added.
#[test]
fn attaching_into_an_empty_buffer_inserts_only_the_reference() {
    assert_eq!(insertion_text("", "@shot.png"), "@shot.png");
}

/// Mid-sentence the `@` needs a word boundary, or the scanner reads it as
/// prose and the chip is never drawn.
#[test]
fn attaching_after_a_word_separates_the_reference() {
    assert_eq!(insertion_text("look at", "@shot.png"), " @shot.png");
}

#[test]
fn attaching_after_a_space_adds_no_second_one() {
    assert_eq!(insertion_text("look at ", "@shot.png"), "@shot.png");
    assert_eq!(insertion_text("line one\n", "@shot.png"),
               "@shot.png",
               "a newline is a boundary too");
}
