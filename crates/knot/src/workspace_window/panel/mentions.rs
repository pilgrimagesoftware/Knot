//! The file listing an agent's `@` lookup completes over: gathering it off
//! the main thread, caching it, and following the folder as it changes.
//!
//! Enumeration reads the filesystem, which must never happen on the render
//! path, so it runs in `spawn_blocking` and leaves its answer in a slot.
//! Nothing on the tokio side has a GPUI context, so nothing there can
//! repaint - `drain_mention_listings` is what turns a landed listing into
//! a frame, and it is called from `repaint_poll_tick` for the same reason
//! every other off-thread source is
//! (`.claude/rules/rust-structure.md`, "Off-thread results must reach a
//! frame").
//!
//! Gathering starts on the agent's first `@`, not when its composer is
//! built: an agent nobody mentions a file to should not cost a walk of its
//! folder. The popup says it is gathering while that runs, which is what
//! `panel-file-mentions` asks for instead of an empty list.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use knot_watch::Watch;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::panel_commands::FolderFiles;
use crate::panel_commands::LookupRegistry;
use crate::workspace_window::WorkspaceWindow;

/// Where an agent's listing has got to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::workspace_window) enum MentionState {
    /// Being walked right now. The popup says so rather than showing an
    /// empty list, which would read as "no such file".
    Gathering,
    /// Listed. `overflowed` says the folder was larger than the cap, so
    /// matching runs over what was gathered and the popup says it is
    /// incomplete.
    Ready { overflowed: bool },
}

/// One agent's mention listing.
pub(in crate::workspace_window) struct PanelMentions {
    state:    MentionState,
    registry: LookupRegistry,
    /// Where `spawn_blocking` leaves a finished listing. Taken by the
    /// repaint poll, which is the only thing that reads it.
    landed:   Arc<Mutex<Option<FolderFiles>>>,
    /// Set by the folder watch when something changed under it.
    dirty:    Arc<AtomicBool>,
    /// Held so it is stopped when the agent's state is torn down; a
    /// dropped `Watch` that nobody stopped keeps its task.
    watch:    Option<Arc<Watch>>,
}

impl PanelMentions {
    /// The entries to complete over. Empty while gathering.
    pub(in crate::workspace_window) fn registry(&self) -> &LookupRegistry {
        &self.registry
    }
}

impl WorkspaceWindow {
    /// Starts `id`'s listing if it has none, and returns how far along it
    /// is.
    ///
    /// Called from the lookup when an `@` token appears, so the first `@`
    /// is what pays for the walk.
    pub(in crate::workspace_window) fn ensure_panel_mentions(&mut self, id: Uuid)
                                                             -> Option<MentionState> {
        if let Some(mentions) = self.panel_mentions.get(&id) {
            return Some(mentions.state);
        }
        let folder = {
            let store = self.store.lock();
            store.agent(id).map(|agent| agent.folder.clone())?
        };

        let landed = Arc::new(Mutex::new(None));
        let dirty = Arc::new(AtomicBool::new(false));
        self.spawn_mention_listing(&folder, Arc::clone(&landed));
        let watch = self.start_mention_watch(&folder, Arc::clone(&dirty));

        self.panel_mentions.insert(id,
                                   PanelMentions { state: MentionState::Gathering,
                                                   registry: LookupRegistry::default(),
                                                   landed,
                                                   dirty,
                                                   watch });
        Some(MentionState::Gathering)
    }

    /// `id`'s listing, if it has one.
    pub(in crate::workspace_window) fn panel_mentions_for(&self, id: Uuid)
                                                          -> Option<&PanelMentions> {
        self.panel_mentions.get(&id)
    }

    /// Walks `folder` off the main thread, leaving the result in `landed`.
    fn spawn_mention_listing(&self, folder: &str, landed: Arc<Mutex<Option<FolderFiles>>>) {
        let folder = folder.to_string();
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
                        let files = tokio::task::spawn_blocking(move || {
                                        FolderFiles::enumerate(std::path::Path::new(&folder))
                                    }).await
                                      .unwrap_or_else(|_| FolderFiles::empty());
                        *landed.lock() = Some(files);
                    });
    }

    /// Watches `folder` so a file created while the agent is open reaches
    /// a later lookup without the agent being reopened.
    ///
    /// Debounced rather than polled: a timer that re-walks a large folder
    /// on a cadence costs the same whether anything changed or not.
    fn start_mention_watch(&self, folder: &str, dirty: Arc<AtomicBool>) -> Option<Arc<Watch>> {
        let watch = Arc::new(Watch::new(folder,
                                        knot_watch::consts::GENERIC_DEBOUNCE,
                                        // Every change matters: a file
                                        // created anywhere under the folder
                                        // is a file that can be mentioned.
                                        |_| true,
                                        move || dirty.store(true, Ordering::SeqCst)));
        let _runtime_guard = self.runtime.enter();
        if let Err(error) = watch.start() {
            // A listing that does not follow the folder is still a
            // listing. Worth saying, not worth refusing to complete over.
            eprintln!("failed to watch {folder} for mention changes: {error}");
            return None;
        }
        Some(watch)
    }

    /// Turns landed listings and fired watches into a repaint.
    ///
    /// Returns whether anything changed. Both reads clear as they go, so
    /// neither may sit on a path that could discard the answer - that is
    /// the exact shape of two of the defects the structure rules record.
    pub(in crate::workspace_window) fn drain_mention_listings(&mut self) -> bool {
        let mut changed = false;
        let mut restart: Vec<(Uuid, Arc<Mutex<Option<FolderFiles>>>)> = Vec::new();

        for (id, mentions) in &mut self.panel_mentions {
            if let Some(files) = mentions.landed.lock().take() {
                mentions.state = MentionState::Ready { overflowed: files.overflowed(), };
                mentions.registry = LookupRegistry::from_sources(&[&files]);
                changed = true;
            }
            // Assigned to a local first: `||` would skip this whenever the
            // take above was `Some`, and because the swap consumes the
            // flag, the skipped change would be lost rather than deferred.
            let fired = mentions.dirty.swap(false, Ordering::SeqCst);
            if fired {
                restart.push((*id, Arc::clone(&mentions.landed)));
            }
        }

        for (id, landed) in restart {
            let folder = {
                let store = self.store.lock();
                store.agent(id).map(|agent| agent.folder.clone())
            };
            if let Some(folder) = folder {
                // The state stays `Ready` with the previous listing while
                // the new walk runs: an agent whose folder changed should
                // keep completing over what it had rather than emptying
                // out mid-sentence.
                self.spawn_mention_listing(&folder, landed);
            }
        }
        changed
    }

    /// Drops `id`'s listing and stops its watch.
    pub(in crate::workspace_window) fn forget_panel_mentions(&mut self, id: Uuid) {
        if let Some(mentions) = self.panel_mentions.remove(&id)
           && let Some(watch) = mentions.watch
        {
            watch.stop();
        }
    }
}
