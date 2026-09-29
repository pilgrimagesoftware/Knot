//! The markdown section's file, read off the render path and re-read when it
//! changes on disk (`openspec/specs/artifact-panel`, "The markdown section
//! follows its file").
//!
//! The section used to call `std::fs::read_to_string` from inside
//! `render_markdown_pane`, once per frame - and GPUI draws a frame per
//! keystroke, so typing in the composer beside an open file re-read it on
//! every key. Re-reading was also the only way an edit reached the screen.
//!
//! Here the read runs on a blocking thread and lands in a slot the render
//! path copies from; a watch on the file schedules the next read, the way
//! `MarkdownPanelView`'s `FileWatcher` does. Landing flips a flag that
//! `repaint_poll_tick` drains through
//! [`WorkspaceWindow::poll_markdown_documents`], so a read that finishes on a
//! quiet window still reaches a frame.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use gpui_kit::SharedString;
use knot_watch::Watch;
use parking_lot::Mutex;
use uuid::Uuid;

use super::review::ReviewState;
use crate::workspace_window::WorkspaceWindow;

/// Where a read's result goes: the text, and the flag that says it is new.
///
/// Cloned into every read - the first one and each one the watch starts - so
/// all of them report through the same slot.
///
/// Reads run on separate blocking threads, so they can finish out of order:
/// one the watch started just before the agent rewrote the file can land
/// after the one started by the rewrite, and would put the old contents
/// back. Each read therefore takes a ticket when it starts, and a result is
/// kept only if no later-started read has already landed. The newest-started
/// read is the one that saw the newest file.
#[derive(Clone, Default)]
struct Landing {
    /// The text, with the ticket of the read that produced it.
    body:   Arc<Mutex<Option<(u64, SharedString)>>>,
    issued: Arc<AtomicU64>,
    landed: Arc<AtomicBool>,
}

impl Landing {
    fn read(&self, path: &Path) {
        let ticket = self.begin();
        self.land(ticket, read_markdown(path).into());
    }

    /// A ticket for a read about to start, later than every one before it.
    fn begin(&self) -> u64 {
        self.issued.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Keeps `text` unless a later-started read has already landed.
    fn land(&self, ticket: u64, text: SharedString) {
        let mut body = self.body.lock();
        if body.as_ref().is_some_and(|(kept, _)| *kept > ticket) {
            return;
        }
        *body = Some((ticket, text));
        drop(body);
        self.landed.store(true, Ordering::Release);
    }
}

/// One agent's open markdown file: its contents once read, the watch that
/// re-reads it, and where the user is in reviewing it.
pub(in crate::workspace_window) struct MarkdownDocument {
    path: PathBuf,
    landing: Landing,
    watch: Option<Watch>,
    /// Kept here rather than beside the panel's arrangement because it is
    /// about this file's contents: a fresh read resets it, as
    /// `MarkdownPanelView.loadContent` does, so a file the agent rewrote
    /// after a review offers Approve and Review again.
    pub(in crate::workspace_window) review: ReviewState,
}

impl MarkdownDocument {
    /// Starts reading `path`, and watching it for the next read.
    ///
    /// Both need the runtime entered: `spawn_blocking` and `Watch::start`
    /// spawn onto it, and the UI thread has no runtime of its own.
    fn open(path: PathBuf, runtime: &tokio::runtime::Runtime) -> Self {
        let _runtime_guard = runtime.enter();
        let landing = Landing::default();
        {
            let (landing, path) = (landing.clone(), path.clone());
            runtime.spawn_blocking(move || landing.read(&path));
        }
        let watch = {
            let (landing, watched) = (landing.clone(), path.clone());
            // Every event counts: the watch is on this one file, so there is
            // nothing else under it to filter out.
            Watch::new(path.clone(),
                       knot_watch::consts::GENERIC_DEBOUNCE,
                       |_| true,
                       move || {
                           let (landing, path) = (landing.clone(), watched.clone());
                           tokio::task::spawn_blocking(move || landing.read(&path));
                       })
        };
        // A file that cannot be watched is still shown; it just does not
        // follow edits made after it opened. Not worth refusing the section
        // for, and the read above says what is wrong if the file is missing.
        let watch = match watch.start() {
            Ok(()) => Some(watch),
            Err(error) => {
                eprintln!("failed to watch {} for changes: {error}", path.display());
                None
            }
        };
        Self { path,
               landing,
               watch,
               review: ReviewState::default() }
    }

    /// The file's contents, or `None` while the first read is in flight.
    pub(in crate::workspace_window) fn body(&self) -> Option<SharedString> {
        self.landing
            .body
            .lock()
            .as_ref()
            .map(|(_, text)| text.clone())
    }

    /// Whether a read has landed since this was last asked. Clears as it
    /// reads.
    fn take_landed(&self) -> bool {
        self.landing.landed.swap(false, Ordering::AcqRel)
    }
}

impl Drop for MarkdownDocument {
    /// `Watch` has no `Drop` of its own, and its task holds the callback, so
    /// a document dropped without this would go on re-reading a file nothing
    /// shows.
    fn drop(&mut self) {
        if let Some(watch) = &self.watch {
            watch.stop();
        }
    }
}

/// `path`'s contents, or a note saying why they could not be read.
///
/// The note is itself markdown, drawn where the file would have been: a
/// section that opened on a missing file should say so rather than sit empty,
/// which reads as still loading.
fn read_markdown(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| {
                                     knot_core::l10n::t_with("artifact_panel.read_failed",
                                                             &[("path",
                                                                &path.display().to_string()),
                                                               ("error", &error.to_string())])
                                 })
}

impl WorkspaceWindow {
    /// Opens a document for every agent in this workspace that has a markdown
    /// file, drops the ones whose file closed or changed, and says whether a
    /// read has landed since the last tick.
    ///
    /// Every agent in the workspace rather than only the selected one, so
    /// selecting an agent with a file open finds it already read instead of
    /// flashing its loading state.
    ///
    /// A landed read resets the review, which is what `MarkdownPanelView`
    /// does on every load.
    pub(in crate::workspace_window) fn poll_markdown_documents(&mut self) -> bool {
        let wanted: Vec<(Uuid, PathBuf)> = {
            let ids = self.workspace_agent_ids();
            let store = self.store.lock();
            ids.into_iter()
               .filter_map(|id| {
                   store.agent(id)
                        .and_then(|agent| agent.markdown_file.clone())
                        .map(|file| (id, file))
               })
               .collect()
        };
        self.markdown_documents.retain(|id, document| {
                                   wanted.iter().any(|(wanted, file)| {
                                                    wanted == id && *file == document.path
                                                })
                               });
        for (id, file) in wanted {
            if !self.markdown_documents.contains_key(&id) {
                let document = MarkdownDocument::open(file, &self.runtime);
                self.markdown_documents.insert(id, document);
            }
        }
        // A loop rather than `any`: `take_landed` clears as it reads, and a
        // short-circuit would leave a later document's flag set for a
        // spurious repaint next tick.
        let mut landed = false;
        for document in self.markdown_documents.values_mut() {
            if document.take_landed() {
                document.review = ReviewState::default();
                landed = true;
            }
        }
        landed
    }

    /// What `id`'s markdown section shows, if it has a document and the first
    /// read has landed. Memory only: this is on the render path.
    pub(in crate::workspace_window) fn markdown_document_body(&self, id: Uuid)
                                                              -> Option<SharedString> {
        self.markdown_documents
            .get(&id)
            .and_then(MarkdownDocument::body)
    }
}

#[cfg(test)]
mod tests;
