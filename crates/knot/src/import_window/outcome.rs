//! What the window says after an import runs.
//!
//! Contract: `openspec/specs/import-ui/spec.md` - "An import always says what
//! it did".
//!
//! Kept separate from the window and its rendering because this is the part
//! that must never be silent, and silence is only testable if the decision of
//! what to say is a function rather than a branch inside a render tree.
//!
//! Every path produces lines. An import that failed says so with the error, an
//! import that did nothing says it did nothing, and an import that succeeded
//! lists what it added, skipped and could not read - there is no combination
//! of inputs that returns an empty summary, which is what left a successful
//! import looking identical to a broken one.

use knot_core::import::{ImportResult, Unreadable};

/// The summary of one import, and whether it failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ImportOutcome {
    pub(super) lines:  Vec<String>,
    /// Drawn in the error colour and kept on screen rather than read as an
    /// ordinary summary.
    pub(super) failed: bool,
}

/// What to show for `result`, folding in anything the scan itself could not
/// read - the user asked to import from a source, and a file that never made
/// the list is part of that answer.
pub(super) fn summarise(result: knot_core::Result<ImportResult>,
                        scanned_unreadable: &[Unreadable])
                        -> ImportOutcome {
    match result {
        Err(error) => ImportOutcome { lines:  vec![knot_core::l10n::t_with("import.failed",
                                                             &[("error", &error.to_string())])],
                                      failed: true, },
        Ok(mut result) => {
            result.unreadable.extend(scanned_unreadable.iter().cloned());
            let lines = result.summary_lines();
            let lines = if lines.is_empty() {
                vec![knot_core::l10n::t("import.nothing_to_do")]
            }
            else {
                lines
            };
            ImportOutcome { lines,
                            failed: false }
        }
    }
}

#[cfg(test)]
mod tests {
    use knot_core::import::{Unreadable, UnreadableReason};

    use super::*;

    /// The defect this file exists for: an import that failed said nothing at
    /// all, because the error went to `eprintln!` and the summary stayed
    /// empty. On screen that is indistinguishable from an import that worked.
    #[test]
    fn a_failed_import_says_so_rather_than_nothing() {
        let outcome = summarise(Err(knot_core::Error::Config("disk is full".into())), &[]);

        assert!(outcome.failed, "a failed import was not marked failed");
        assert_eq!(outcome.lines.len(), 1);
        assert!(outcome.lines[0].contains("disk is full"),
                "the error itself is missing from the summary: {:?}",
                outcome.lines);
    }

    /// An import with nothing to do is a real answer and must be given, not
    /// left as a blank panel the user has to interpret.
    #[test]
    fn an_import_that_did_nothing_says_it_did_nothing() {
        let outcome = summarise(Ok(ImportResult::default()), &[]);

        assert!(!outcome.failed);
        assert_eq!(outcome.lines,
                   vec![knot_core::l10n::t("import.nothing_to_do")]);
        assert_ne!(outcome.lines[0], "import.nothing_to_do",
                   "the key is missing from the catalog");
    }

    #[test]
    fn a_successful_import_reports_what_it_added() {
        let result = ImportResult { added: vec!["SRE".into()],
                                    skipped: vec!["WIP".into()],
                                    ..ImportResult::default() };

        let outcome = summarise(Ok(result), &[]);

        assert!(!outcome.failed);
        assert_eq!(outcome.lines.len(), 2, "{:?}", outcome.lines);
        assert!(outcome.lines.iter().any(|line| line.contains("SRE")));
        assert!(outcome.lines.iter().any(|line| line.contains("WIP")));
    }

    /// What the scan could not read is part of the answer even though the
    /// import itself never saw those files.
    #[test]
    fn files_the_scan_could_not_read_are_named_in_the_summary() {
        let scanned = [Unreadable { name:   "broken.md".into(),
                                    reason: UnreadableReason::NoFrontmatter, }];

        let outcome = summarise(Ok(ImportResult::default()), &scanned);

        assert!(!outcome.failed);
        assert!(outcome.lines.iter().any(|line| line.contains("broken.md")),
                "the unreadable file went unmentioned: {:?}",
                outcome.lines);
    }
}
