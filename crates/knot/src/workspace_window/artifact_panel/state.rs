//! The artifact panel's per-agent arrangement, and the reads and writes the
//! render path makes of it (`openspec/specs/artifact-panel`, "Panel
//! arrangement is discarded with the window").
//!
//! On the window rather than in settings on two counts: it is keyed by agent
//! while `WorkspaceUiState` is keyed by workspace, and the git panel's width
//! already works this way. Persisting it would make the artifact panel the
//! only side panel whose arrangement outlives its window.

use std::path::PathBuf;

use uuid::Uuid;

use super::layout;
use crate::consts;
use crate::workspace_window::WorkspaceWindow;

/// One agent's artifact panel arrangement.
///
/// `expanded` is the one field not simply defaulted: it is taken from
/// `display-markdown`'s `maximized` argument, tracked alongside the pair it
/// was taken from so a later call that says something new is noticed. See
/// [`ArtifactPanelArrangement::reseed_expanded`].
#[derive(Debug, Clone)]
pub(in crate::workspace_window) struct ArtifactPanelArrangement {
    pub(in crate::workspace_window) width: f32,
    pub(in crate::workspace_window) split: f32,
    pub(in crate::workspace_window) markdown_collapsed: bool,
    pub(in crate::workspace_window) mermaid_collapsed: bool,
    pub(in crate::workspace_window) expanded: bool,
    /// The `(file, maximized)` pair `expanded` was last taken from, or `None`
    /// before any has been. Compared rather than remembered as a flag: what
    /// makes a call worth acting on is that it differs from the last one.
    seeded_from: Option<(Option<PathBuf>, bool)>,
}

impl Default for ArtifactPanelArrangement {
    fn default() -> Self {
        Self { width:              consts::ARTIFACT_PANEL_DEFAULT_WIDTH,
               split:              consts::ARTIFACT_PANEL_DEFAULT_SPLIT,
               markdown_collapsed: false,
               mermaid_collapsed:  false,
               expanded:           false,
               seeded_from:        None, }
    }
}

impl ArtifactPanelArrangement {
    /// Takes `expanded` from `maximized` when this call says something the
    /// last one did not.
    ///
    /// Keyed on the `(file, maximized)` pair rather than the file alone:
    /// Swift triggers on the file changing
    /// (`ContentView.swift:113-117`, inside `.onChange(of:
    /// activeAgent?.markdownFilePath)`), which does not fire for an equal
    /// value and so strands a re-show of the open file asking to be
    /// maximized. Keying on every call instead would collapse a panel the
    /// user expanded by hand whenever an agent re-shows a file it edited.
    ///
    /// Returns whether anything moved, so the caller can notify.
    fn reseed_expanded(&mut self, file: Option<&PathBuf>, maximized: bool) -> bool {
        let pair = (file.cloned(), maximized);
        if self.seeded_from.as_ref() == Some(&pair) {
            return false;
        }
        self.seeded_from = Some(pair);
        // Only a file carries the argument. A call that cleared the file says
        // nothing about size, and letting it write `false` here would collapse
        // a panel whose diagram is still open - the trap
        // `clear_markdown_panel` sets by resetting `markdown_maximized`
        // itself (`knot-agents/src/store/panels.rs:24`).
        if file.is_none() {
            return false;
        }
        let moved = self.expanded != maximized;
        self.expanded = maximized;
        moved
    }
}

/// What the artifact fields held when the window last drew them.
///
/// Compared each poll because nothing else in `repaint_poll_tick`'s chain
/// reads them, and the MCP tools write them from a thread with no GPUI
/// context to notify from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(in crate::workspace_window) struct ArtifactSnapshot {
    pub(in crate::workspace_window) markdown:  Option<PathBuf>,
    pub(in crate::workspace_window) maximized: bool,
    pub(in crate::workspace_window) mermaid:   Option<String>,
}

impl WorkspaceWindow {
    /// `id`'s arrangement, at its defaults if the panel has not been touched.
    pub(in crate::workspace_window) fn artifact_arrangement(&self, id: Uuid)
                                                            -> ArtifactPanelArrangement {
        self.artifact_panel.get(&id).cloned().unwrap_or_default()
    }

    /// Sets `id`'s panel width, clamped to the bounds a drag may reach.
    pub(in crate::workspace_window) fn set_artifact_panel_width(&mut self, id: Uuid, width: f32) {
        self.artifact_panel.entry(id).or_default().width = layout::clamp_width(width);
    }

    /// Sets `id`'s section split, clamped to the bounds a drag may reach.
    pub(in crate::workspace_window) fn set_artifact_split(&mut self, id: Uuid, ratio: f32) {
        self.artifact_panel.entry(id).or_default().split = layout::clamp_split(ratio);
    }

    /// Toggles whether `id`'s markdown section is collapsed.
    pub(in crate::workspace_window) fn toggle_artifact_markdown_collapsed(&mut self, id: Uuid) {
        let entry = self.artifact_panel.entry(id).or_default();
        entry.markdown_collapsed = !entry.markdown_collapsed;
    }

    /// Toggles whether `id`'s mermaid section is collapsed.
    pub(in crate::workspace_window) fn toggle_artifact_mermaid_collapsed(&mut self, id: Uuid) {
        let entry = self.artifact_panel.entry(id).or_default();
        entry.mermaid_collapsed = !entry.mermaid_collapsed;
    }

    /// Toggles whether `id`'s panel is expanded over the content pane.
    ///
    /// Writes the window's own flag and never `Agent::markdown_maximized`:
    /// that field is what an agent asked for, and a control the user toggled
    /// must not be mistakable for an instruction an agent gave.
    pub(in crate::workspace_window) fn toggle_artifact_expanded(&mut self, id: Uuid) {
        let entry = self.artifact_panel.entry(id).or_default();
        entry.expanded = !entry.expanded;
    }

    /// Whether `id`'s panel is expanded.
    ///
    /// The live state, which the user's toggle changes - not the `maximized`
    /// argument last recorded. The two diverge the moment the user collapses
    /// a panel an agent maximized, and it is what is on screen that decides
    /// whether there is a composer to focus.
    pub(in crate::workspace_window) fn artifact_panel_expanded(&self, id: Uuid) -> bool {
        self.artifact_panel
            .get(&id)
            .is_some_and(|entry| entry.expanded)
    }

    /// Drops every trace of `id`'s panel, with the agent.
    pub(in crate::workspace_window) fn forget_artifact_panel(&mut self, id: Uuid) {
        self.artifact_panel.remove(&id);
        self.artifact_panel_resize.remove(&id);
        self.artifact_split_resize.remove(&id);
    }

    /// Reconciles `id`'s arrangement with what the store now holds, and says
    /// whether anything moved.
    ///
    /// Two jobs, both of which have to happen before the frame is assembled:
    /// take `expanded` from a `display-markdown` call that said something
    /// new, and reset the arrangement once the agent has no artifact left.
    pub(in crate::workspace_window) fn reconcile_artifact_panel(&mut self, id: Uuid,
                                                                snapshot: &ArtifactSnapshot)
                                                                -> bool {
        // Neither artifact left: the panel is gone, and its arrangement goes
        // with it so the next one opens at the defaults. Checked on the
        // snapshot rather than on `markdown_maximized`, which
        // `clear_markdown_panel` has already set false by now.
        if snapshot.markdown.is_none() && snapshot.mermaid.is_none() {
            return self.artifact_panel.remove(&id).is_some();
        }
        let entry = self.artifact_panel.entry(id).or_default();
        let moved = entry.reseed_expanded(snapshot.markdown.as_ref(), snapshot.maximized);
        // A section that closed leaves its collapse behind, so reopening it
        // shows it expanded rather than as a header the user has to find.
        if snapshot.markdown.is_none() && entry.markdown_collapsed {
            entry.markdown_collapsed = false;
        }
        if snapshot.mermaid.is_none() && entry.mermaid_collapsed {
            entry.mermaid_collapsed = false;
        }
        moved
    }
}

#[cfg(test)]
mod tests;
