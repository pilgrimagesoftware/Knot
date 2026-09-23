//! Turning a [`RepoStatus`] into the four sections the panel draws, and
//! deciding whether a selection survived a refresh.
//!
//! Pure functions over a status, kept apart from the element tree on purpose:
//! this is the part with the edge cases worth testing - a path that is staged
//! and then modified again belongs in two sections at once - and it must not
//! need a window to exercise.
//!
//! Contract: `openspec/specs/git-panel-ui/spec.md`.

use std::path::{Path, PathBuf};

use knot_git::{ChangeType, FileEntry, RepoStatus};

use super::state::Selection;

/// Which of the four groups a row belongs to. Also the order they are drawn
/// in, which is why it derives `Ord`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) enum SectionKind {
    Staged,
    Unstaged,
    Untracked,
    Conflicted,
}

// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
impl SectionKind {
    /// Whether a row in this section is an index-side row. Drives which diff
    /// the row shows and which actions it offers.
    pub(crate) fn is_staged(self) -> bool {
        self == Self::Staged
    }

    /// The l10n key for this section's title.
    pub(crate) fn title_key(self) -> &'static str {
        match self {
            Self::Staged => "git_panel.section.staged",
            Self::Unstaged => "git_panel.section.unstaged",
            Self::Untracked => "git_panel.section.untracked",
            Self::Conflicted => "git_panel.section.conflicted",
        }
    }
}

/// One drawable row: a path on one side of the status.
#[derive(Debug, Clone, PartialEq, Eq)]
// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) struct Row {
    pub(crate) path:      PathBuf,
    pub(crate) orig_path: Option<PathBuf>,
    /// The change type for *this row's* side.
    ///
    /// Swift derived one glyph per path, preferring the staged side, so the
    /// unstaged row of a staged-and-modified path showed the staged glyph.
    /// Each row shows its own side here, which is what the row is about.
    pub(crate) change:    ChangeType,
    pub(crate) section:   SectionKind,
}

// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
impl Row {
    pub(crate) fn selection(&self) -> Selection {
        Selection { path:      self.path.clone(),
                    orig_path: self.orig_path.clone(),
                    staged:    self.section.is_staged(), }
    }

    /// The final path component, or the whole path when it has no parent.
    pub(crate) fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.to_string_lossy().into_owned())
    }

    /// The containing directory, or `None` at the repository root.
    pub(crate) fn directory(&self) -> Option<String> {
        self.path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_string_lossy().into_owned())
    }
}

/// A section with entries. A section with none is never built, so the panel
/// has nothing to omit at draw time.
#[derive(Debug, Clone, PartialEq, Eq)]
// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) struct Section {
    pub(crate) kind: SectionKind,
    pub(crate) rows: Vec<Row>,
}

/// Groups a status into sections, in draw order, omitting empty ones.
///
/// The sections are independent filters, not a partition: a path with both a
/// staged and an unstaged change yields a row in each. Conflicted entries are
/// taken out first, so a conflict is listed once as a conflict rather than
/// twice as an unmerged change on both sides.
// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) fn group(status: &RepoStatus) -> Vec<Section> {
    let conflicted: Vec<Row> = status.conflicted()
                                     .map(|e| row(e, ChangeType::Unmerged, SectionKind::Conflicted))
                                     .collect();
    let is_conflict = |entry: &FileEntry| conflicted.iter().any(|r| r.path == entry.path);

    let staged = status.staged()
                       .filter(|e| !is_conflict(e))
                       .filter_map(|e| e.staged.map(|c| row(e, c, SectionKind::Staged)))
                       .collect();
    let unstaged = status.modified()
                         .filter(|e| !is_conflict(e))
                         .filter_map(|e| e.unstaged.map(|c| row(e, c, SectionKind::Unstaged)))
                         .collect();
    let untracked = status.untracked()
                          .filter(|e| !is_conflict(e))
                          .map(|e| row(e, ChangeType::Untracked, SectionKind::Untracked))
                          .collect();

    [Section { kind: SectionKind::Staged,
               rows: staged, },
     Section { kind: SectionKind::Unstaged,
               rows: unstaged, },
     Section { kind: SectionKind::Untracked,
               rows: untracked, },
     Section { kind: SectionKind::Conflicted,
               rows: conflicted, }].into_iter()
                                   .filter(|s| !s.rows.is_empty())
                                   .collect()
}

// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
fn row(entry: &FileEntry, change: ChangeType, section: SectionKind) -> Row {
    Row { path: entry.path.clone(),
          orig_path: entry.orig_path.clone(),
          change,
          section }
}

/// Whether a selection still names a row in `sections`.
///
/// Both halves of the pair must match. Swift checked only the path, so a file
/// that moved from one side to the other kept a diff that no longer described
/// it.
// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) fn selection_survives(sections: &[Section], selection: &Selection) -> bool {
    sections.iter().any(|section| {
                       section.kind.is_staged() == selection.staged
                       && section.rows.iter().any(|r| r.path == selection.path)
                   })
}

/// Every path in a section, as the `&str` slice the git operations take.
///
/// Returns `None` when any path is not valid UTF-8 rather than silently
/// dropping it: a bulk action that quietly skipped a file would report success
/// having left it behind.
// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) fn paths_of(rows: &[Row]) -> Option<Vec<&str>> {
    rows.iter().map(|r| r.path.to_str()).collect()
}

/// Whether `path` is worth showing as its own directory line.
// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) fn has_directory(path: &Path) -> bool {
    path.parent().is_some_and(|p| !p.as_os_str().is_empty())
}
