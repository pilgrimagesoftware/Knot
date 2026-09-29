//! What the user closed in an agent's artifact panel, and bringing it back
//! (`openspec/specs/artifact-panel`, "A closed panel can be reopened").
//!
//! Closing a section clears the agent's artifact in the store - that is what
//! makes the panel go away, and what the rest of the window reads. Before
//! this, nothing remembered what had been there: the markdown file survived
//! only in the agent's "Markdown Files" history, and a diagram's source was
//! dropped outright, so the only way back was for the agent to call
//! `display-markdown` or `view-mermaid` again.
//!
//! Held on the window rather than in the store, like the rest of the panel's
//! state: only this window's close controls write it, so only this window
//! has anything to reopen.

use std::path::PathBuf;

use uuid::Uuid;

use super::state::ArtifactSnapshot;
use crate::workspace_window::WorkspaceWindow;

/// Which of an agent's sections a close control closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::workspace_window) enum CloseTarget {
    Markdown,
    Mermaid,
    /// The toolbar's close-all.
    Both,
}

impl CloseTarget {
    const fn markdown(self) -> bool {
        matches!(self, Self::Markdown | Self::Both)
    }

    const fn mermaid(self) -> bool {
        matches!(self, Self::Mermaid | Self::Both)
    }
}

/// A diagram as it stood when it was closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workspace_window) struct ClosedDiagram {
    pub(in crate::workspace_window) source: String,
    pub(in crate::workspace_window) title:  Option<String>,
}

/// The last markdown file and diagram the user closed for one agent.
///
/// One slot per section rather than a history: reopening restores the panel
/// the user just dismissed, and the "Markdown Files" menu already covers
/// going further back for files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(in crate::workspace_window) struct ClosedArtifacts {
    pub(in crate::workspace_window) markdown: Option<PathBuf>,
    pub(in crate::workspace_window) mermaid:  Option<ClosedDiagram>,
}

impl ClosedArtifacts {
    /// What reopening would restore given what is open now: a closed section
    /// whose slot is empty again.
    ///
    /// A section the agent has since filled is left alone. Its artifact is
    /// newer than the one the user closed, and reopening must not replace
    /// it.
    pub(in crate::workspace_window) fn reopenable(&self, open: &ArtifactSnapshot)
                                                  -> ClosedArtifacts {
        ClosedArtifacts { markdown: self.markdown.clone().filter(|_| open.markdown.is_none()),
                          mermaid:  self.mermaid.clone().filter(|_| open.mermaid.is_none()), }
    }

    pub(in crate::workspace_window) fn is_empty(&self) -> bool {
        self.markdown.is_none() && self.mermaid.is_none()
    }
}

impl WorkspaceWindow {
    /// Closes `target`'s sections of `id`'s panel, remembering what they held
    /// so [`Self::reopen_artifacts`] can bring it back.
    ///
    /// The one path every close control takes - a section's own and the
    /// toolbar's close-all - so none of them can clear an artifact without
    /// recording it.
    pub(in crate::workspace_window) fn close_artifact_sections(&mut self, id: Uuid,
                                                               target: CloseTarget) {
        let mut store = self.store.lock();
        let Some(agent) = store.agent(id)
        else {
            return;
        };
        let markdown = agent.markdown_file.clone().filter(|_| target.markdown());
        let mermaid = agent.mermaid_source
                           .clone()
                           .filter(|_| target.mermaid())
                           .map(|source| ClosedDiagram { source,
                                                         title: agent.mermaid_title.clone() });
        if markdown.is_none() && mermaid.is_none() {
            return;
        }
        let closed = self.artifact_closed.entry(id).or_default();
        if let Some(file) = markdown {
            closed.markdown = Some(file);
            if let Err(error) = store.clear_markdown_panel(id) {
                eprintln!("failed to close the markdown section: {error}");
            }
        }
        if let Some(diagram) = mermaid {
            closed.mermaid = Some(diagram);
            if let Err(error) = store.clear_mermaid_panel(id) {
                eprintln!("failed to close the diagram section: {error}");
            }
        }
    }

    /// What reopening `id`'s panel would restore, empty when there is
    /// nothing to bring back.
    ///
    /// Memory and one store read, no I/O, so the header can ask it every
    /// frame to decide whether to draw the control.
    pub(in crate::workspace_window) fn reopenable_artifacts(&self, id: Uuid) -> ClosedArtifacts {
        self.artifact_closed
            .get(&id)
            .map(|closed| closed.reopenable(&self.artifact_snapshot(id)))
            .unwrap_or_default()
    }

    /// Puts back what the user closed in `id`'s panel.
    ///
    /// Through the store's own setters, so the panel reappears by the same
    /// path an agent's call takes and the repaint poll sees it the same way.
    /// The markdown file is restored unmaximized: `maximized` is what the
    /// agent asked for, and the user reopening a panel asks for nothing
    /// about its size.
    pub(in crate::workspace_window) fn reopen_artifacts(&mut self, id: Uuid) {
        let restore = self.reopenable_artifacts(id);
        if restore.is_empty() {
            return;
        }
        let mut store = self.store.lock();
        if let Some(file) = restore.markdown
           && let Err(error) = store.set_markdown_panel(id, file, false)
        {
            eprintln!("failed to reopen the markdown section: {error}");
        }
        if let Some(diagram) = restore.mermaid
           && let Err(error) = store.set_mermaid_panel(id, diagram.source, diagram.title)
        {
            eprintln!("failed to reopen the diagram section: {error}");
        }
    }
}

#[cfg(test)]
mod tests;
