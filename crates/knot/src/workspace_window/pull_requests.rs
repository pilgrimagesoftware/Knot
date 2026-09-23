//! Collecting what the two detection taps saw, and recording it.
//!
//! Contract: the `pull-request-tracking` capability spec under
//! `openspec/changes/pull-request-tracking/specs/`.
//!
//! Neither tap can record on its own: `Grid` is a parser and `PanelState` is
//! pure folded state, and neither knows which agent it belongs to or holds the
//! store. Both buffer what they saw; this is where the two meet the agent id
//! and become one record, so "a panel agent opened a pull request" and "a
//! terminal agent opened one" cannot drift apart.
//!
//! Drained every poll tick for *every* agent in the workspace, not only the
//! selected one: an agent working in an unselected pane is exactly the case
//! this feature exists for.

use gpui_kit::App;
use uuid::Uuid;

use super::WorkspaceWindow;
use crate::panel_session::PanelSessionSlot;

impl WorkspaceWindow {
    /// Drain both taps and record whatever they saw, returning whether
    /// anything was new.
    ///
    /// Persists only when a record was actually added. A URL scrolling past
    /// for the second time writes nothing, which is what keeps a chatty agent
    /// from rewriting a settings document on every poll.
    pub(super) fn drain_pull_requests(&mut self, cx: &App) -> bool {
        let mut seen: Vec<(Uuid, String)> = Vec::new();

        for (id, session) in &self.sessions {
            let Some(grid) = session.lock().grid()
            else {
                continue;
            };
            let urls = grid.lock().take_pull_request_urls();
            seen.extend(urls.into_iter().map(|url| (*id, url)));
        }

        for (id, slot) in &self.panel_sessions {
            let slot = slot.lock();
            let PanelSessionSlot::Ready(handle) = &*slot
            else {
                continue;
            };
            let urls = handle.state().lock().take_pull_request_urls();
            seen.extend(urls.into_iter().map(|url| (*id, url)));
        }

        if seen.is_empty() {
            return false;
        }

        let mut recorded = false;
        {
            let mut store = self.store.lock();
            for (agent_id, url) in seen {
                recorded |= store.record_pull_request(agent_id, url);
            }
        }
        if recorded {
            self.persist_pull_requests(cx);
        }
        recorded
    }

    /// Write the store's recorded pull requests back to settings.
    ///
    /// The store guard is released before the write, the same as
    /// `persist_agents`: the write is blocking I/O and nothing else should
    /// wait on the store while it runs.
    pub(super) fn persist_pull_requests(&mut self, cx: &App) {
        let installed = {
            let store = self.store.lock();
            crate::settings_global::write(cx, |settings| {
                settings.pull_requests = store.pull_requests().to_vec();
            })
        };
        if let Err(error) = installed.persist_pull_requests() {
            // Not surfaced: the document is written after every change, so
            // the next sighting retries, and a dialog per URL would be worse
            // than the loss it warns about. Logged because a failure here is
            // what makes the list come back short after a relaunch.
            eprintln!("failed to persist recorded pull requests: {error}");
        }
    }
}
