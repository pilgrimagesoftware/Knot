//! The screenshots a report names (#566).
//!
//! GitHub has no API for uploading a file with an issue, and an image cannot
//! ride in the body as a log does. So a report *names* its screenshots, and
//! delivery puts the files one drag away: the filed issue's page opens and
//! Finder shows them, or Finder shows them beside the compose page. Knot
//! never reads the files - it holds paths, and the body carries names.
//!
//! Everything here is pure, and tested without a picker or a file.

use std::path::{Path, PathBuf};

use crate::consts::{BUG_REPORT_MAX_SCREENSHOTS, BUG_REPORT_SCREENSHOT_EXTENSIONS};

/// Appends the image files among `picked` to `chosen`, up to
/// [`BUG_REPORT_MAX_SCREENSHOTS`], returning how many were skipped - not an
/// image, or past the cap. A file already chosen is neither added twice nor
/// counted as skipped: picking it again changed nothing.
pub(super) fn add(chosen: &mut Vec<PathBuf>, picked: impl IntoIterator<Item = PathBuf>) -> usize {
    let mut skipped = 0;
    for path in picked {
        if chosen.contains(&path) {
            continue;
        }
        if is_image(&path) && chosen.len() < BUG_REPORT_MAX_SCREENSHOTS {
            chosen.push(path);
        }
        else {
            skipped += 1;
        }
    }
    skipped
}

/// Whether `path` names an image type GitHub renders inline, by extension.
fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            BUG_REPORT_SCREENSHOT_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
        })
}

/// What the dialog and the body show for `path`: its file name. Not the full
/// path, which names the user's home directory and says nothing a reader of
/// the issue needs.
pub(super) fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(),
                                 |name| name.to_string_lossy().into_owned())
}

/// The body's screenshots section, or nothing when there are none: a heading,
/// a line saying the reporter adds them, and each file's name.
pub(super) fn body_section(screenshots: &[PathBuf]) -> String {
    if screenshots.is_empty() {
        return String::new();
    }
    let names = screenshots.iter()
                           .map(|path| format!("- `{}`\n", file_name(path)))
                           .collect::<String>();
    format!("\n### {}\n\n{}\n\n{names}",
            knot_core::l10n::t("bug_report.screenshots.label"),
            knot_core::l10n::t("bug_report.screenshots.body_note"))
}

/// How a report's screenshots reach the issue, which decides what the
/// dialog tells the user to do with them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Handoff {
    /// The report has no screenshots.
    None,
    /// Finder shows them, ready to drag onto the page.
    Shown,
    /// This platform has no file manager to show them in; the body still
    /// names them, and the user attaches them by hand.
    ByHand,
}

impl Handoff {
    /// The hand-off for `count` screenshots on a platform that can, or
    /// cannot, reveal files.
    pub(super) fn for_report(count: usize, can_reveal: bool) -> Self {
        match (count, can_reveal) {
            (0, _) => Self::None,
            (_, true) => Self::Shown,
            (_, false) => Self::ByHand,
        }
    }
}

#[cfg(test)]
mod tests;
