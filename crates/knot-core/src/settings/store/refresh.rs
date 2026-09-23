//! Re-reading the preferences document into a `Settings` that already
//! exists.
//!
//! Its own module because it is the one read that is *not* a load: every
//! other path here builds a `Settings` from nothing, and this one has to
//! decide, field by kind, what a value already in memory is allowed to lose.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.

use super::Settings;
use super::documents;
use crate::error::Result;

impl Settings {
    /// Re-read the preferences document in place, keeping every collection
    /// this value already holds.
    ///
    /// For a window that took a copy of the settings surface when it opened
    /// and needs the scalars a *later* write put on disk. Only the scalars
    /// move: the durable collections are `#[serde(skip)]`, each lives in its
    /// own document, and a holder's roster may legitimately be ahead of the
    /// file it was last written to. Pulling those back from disk here would
    /// be the read-side twin of the write-side lost update.
    ///
    /// The fields transplanted below are exactly the `#[serde(skip)]` set,
    /// which is also the set [`Settings::read_documents`] fills from their own
    /// files. A collection added to one and forgotten here would be cleared
    /// the next time a preference changed, so
    /// `reload_preferences_keeps_every_collection` asserts the two stay in
    /// step.
    ///
    /// Tolerant on the same terms as [`Settings::load`]: a missing or
    /// unreadable preferences document yields that document's defaults rather
    /// than an error, so a refresh cannot be what takes a running app down.
    ///
    /// See `openspec/specs/settings-persistence/spec.md`.
    pub fn reload_preferences(&mut self) -> Result<()> {
        let paths = self.resolved_paths()?;
        let mut fresh = match documents::read_object(&paths.preferences()) {
            Some(value) => Self::from_preferences(value),
            None => Self::default(),
        };
        fresh.saved_agents = std::mem::take(&mut self.saved_agents);
        fresh.saved_workspaces = std::mem::take(&mut self.saved_workspaces);
        fresh.workspace_ui = std::mem::take(&mut self.workspace_ui);
        fresh.personas = std::mem::take(&mut self.personas);
        fresh.bench_agents = std::mem::take(&mut self.bench_agents);
        fresh.recent_repos = std::mem::take(&mut self.recent_repos);
        fresh.pull_requests = std::mem::take(&mut self.pull_requests);
        fresh.paths = Some(paths);
        *self = fresh;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
