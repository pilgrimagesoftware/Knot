//! What one frame of the panel needs, flattened out of the caches before any
//! element is built.
//!
//! Same discipline as `pull_requests_view`: read the caches once, copy out
//! what the frame draws, and build the element tree without holding a lock
//! across it.

use knot_git::ChangeType;
use uuid::Uuid;

use crate::git_panel::sections::{Section, group};
use crate::git_panel::state::{DiffOutcome, GitStatusSnapshot, Selection};
use crate::workspace_window::WorkspaceWindow;

/// The panel's content for one frame, in the order the states are chosen:
/// nothing has landed yet, the folder is not a repository, the read failed,
/// the tree is clean, or there are changes.
pub(super) enum PanelContent {
    Loading,
    NotARepository,
    Failed(String),
    Clean,
    Changes(Box<TreeView>),
}

/// A working tree with changes, and the branch line above it.
pub(super) struct TreeView {
    pub(super) branch:   Option<String>,
    pub(super) ahead:    u32,
    pub(super) behind:   u32,
    pub(super) sections: Vec<Section>,
    /// Whether anything is staged, which is what gates the commit control.
    pub(super) staged:   bool,
}

/// Everything one frame of the panel draws.
pub(super) struct PanelView {
    pub(super) agent:     Uuid,
    pub(super) folder:    String,
    pub(super) width:     f32,
    pub(super) content:   PanelContent,
    pub(super) selection: Option<Selection>,
    /// The diff for the current selection, or `None` when none has landed.
    /// Only ever the current selection's: a reply for a row the user clicked
    /// away from lives under its own key and is never read here.
    pub(super) diff:      Option<DiffOutcome>,
    pub(super) error:     Option<String>,
}

impl WorkspaceWindow {
    /// Builds this frame's panel view for `id`, claiming any read that has
    /// aged out on the way.
    ///
    /// Claiming here rather than in a separate pass keeps the two in step:
    /// the frame asks for exactly what it is about to draw, and the claim is
    /// a map lookup and an `Instant` compare, not a subprocess.
    pub(super) fn git_panel_view(&mut self, id: Uuid, folder: &str) -> PanelView {
        self.refresh_git_status(id, folder);

        let snapshot = self.git_status.get(&id);
        let content = match snapshot {
            None => PanelContent::Loading,
            Some(GitStatusSnapshot::NotARepository) => PanelContent::NotARepository,
            Some(GitStatusSnapshot::Failed(reason)) => PanelContent::Failed(reason),
            Some(GitStatusSnapshot::Loaded(status)) => {
                let sections = group(&status);
                if sections.is_empty() {
                    PanelContent::Clean
                }
                else {
                    let staged = sections.iter().any(|s| s.kind.is_staged());
                    PanelContent::Changes(Box::new(TreeView { branch: status.head.clone(),
                                                              ahead: status.ahead,
                                                              behind: status.behind,
                                                              sections,
                                                              staged }))
                }
            }
        };

        self.prune_git_selection(id, &content);

        let selection = self.git_selection.get(&id).cloned();
        let diff = selection.as_ref().and_then(|selection| {
                                         self.refresh_git_diff(id, folder, selection);
                                         self.git_diffs.get(&selection.key(id))
                                     });

        PanelView { agent: id,
                    folder: folder.to_string(),
                    width: self.git_panel_width(id),
                    content,
                    selection,
                    diff,
                    error: self.git_action_error.get(&id).cloned() }
    }

    /// Clears a selection whose row is no longer in the tree.
    ///
    /// Both halves of the pair must still match. Swift checked only the path,
    /// so a file that moved from one side to the other kept a diff that no
    /// longer described it.
    fn prune_git_selection(&mut self, id: Uuid, content: &PanelContent) {
        let Some(selection) = self.git_selection.get(&id)
        else {
            return;
        };

        let survives = match content {
            PanelContent::Changes(tree) => {
                crate::git_panel::sections::selection_survives(&tree.sections, selection)
            }
            // Nothing has landed yet, so there is nothing to disagree with
            // and the selection stands.
            PanelContent::Loading => true,
            _ => false,
        };

        if !survives {
            let key = selection.key(id);
            self.git_selection.remove(&id);
            self.git_diffs.forget(&key);
        }
    }

    pub(super) fn git_panel_width(&self, id: Uuid) -> f32 {
        self.git_panel_width
            .get(&id)
            .copied()
            .unwrap_or(crate::consts::GIT_PANEL_DEFAULT_WIDTH)
    }
}

/// The one-character glyph for a change type, as git's own status codes write
/// it. Not localized: these are git's letters, and a translated `M` would
/// stop matching what `git status` prints.
pub(super) fn change_glyph(change: ChangeType) -> &'static str {
    match change {
        ChangeType::Modified => "M",
        ChangeType::Added => "A",
        ChangeType::Deleted => "D",
        ChangeType::Renamed => "R",
        ChangeType::Copied => "C",
        ChangeType::Untracked => "?",
        ChangeType::Unmerged => "U",
        ChangeType::Ignored => "!",
    }
}
