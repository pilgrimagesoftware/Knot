//! What the Pull Requests view asks of the window: the rows to draw, the
//! breakdown the sidebar row shows, the refreshes that fill them in, and the
//! two things the user can do with a row.
//!
//! Contract: the `pull-request-tracking` capability spec under
//! `openspec/changes/pull-request-tracking/specs/`.
//!
//! Recording is `super::pull_requests`; this is the reading half.

use gpui_kit::component::WindowExt;
use gpui_kit::{Context, Window};
use knot_forge::GhRunner;
use uuid::Uuid;

use super::WorkspaceWindow;
use crate::pull_request_state::{self, PullRequestCounts};
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::render::pull_requests_pane::{PullRequestGroup, PullRequestRow};

impl WorkspaceWindow {
    /// This workspace's recorded pull request URLs, newest first.
    pub(super) fn workspace_pull_request_urls(&self) -> Vec<String> {
        self.store
            .lock()
            .pull_requests_for_workspace(self.workspace_id)
            .into_iter()
            .map(|record| record.url.clone())
            .collect()
    }

    /// How this workspace's records break down by state, for the sidebar row.
    pub(super) fn pull_request_counts(&self) -> PullRequestCounts {
        pull_request_state::counts_for(&self.pull_request_states.snapshot(),
                                       &self.workspace_pull_request_urls())
    }

    /// The rows the pane draws, grouped by the agent that opened them.
    ///
    /// Flattened out of the store and the cache here, so neither lock is held
    /// while the element tree is built.
    pub(super) fn pull_request_groups(&self) -> Vec<PullRequestGroup> {
        let states = self.pull_request_states.snapshot();
        let mut groups: Vec<PullRequestGroup> = Vec::new();
        let store = self.store.lock();
        for record in store.pull_requests_for_workspace(self.workspace_id) {
            let row = PullRequestRow { url:   record.url.clone(),
                                       state: states.get(&record.url).cloned().flatten(), };
            match groups.iter_mut()
                        .find(|group| group.agent_id == record.agent_id)
            {
                Some(group) => group.rows.push(row),
                None => {
                    // An agent that has gone is not a group: its records go
                    // with it, so this is only reachable mid-removal.
                    let Some(agent) = store.agent(record.agent_id)
                    else {
                        continue;
                    };
                    groups.push(PullRequestGroup { agent_id: record.agent_id,
                                                   agent:    agent.name.clone(),
                                                   rows:     vec![row], });
                }
            }
        }
        groups
    }

    /// Fetch what has aged out, if the view is showing.
    ///
    /// Gated on the view rather than run on a timer: a workspace with fifty
    /// recorded pull requests must not be polling `gh` fifty times while the
    /// user is doing something else. Nothing here runs a subprocess - each
    /// claim hands its writer to `spawn_blocking` and a later frame draws the
    /// answer. That includes the availability probe, which is `gh auth
    /// status` and was the one thing here that did run on the frame.
    pub(super) fn refresh_pull_request_states(&mut self) {
        if self.view_mode != WorkspaceViewMode::PullRequests {
            return;
        }
        // One probe before twenty lookups: if `gh` is absent or signed out,
        // every one of them would fail the same way, and the view says so
        // once instead.
        if let Some(writer) = self.forge_status
                                  .claim_probe(pull_request_state::PROBE_MAX_AGE)
        {
            self.runtime
                .spawn_blocking(move || writer.record(knot_forge::probe()));
        }
        // Nothing to fetch until the first probe lands, and nothing to fetch
        // after one that found no usable `gh`. Either way a later frame
        // arrives here again, because the probe marks the view for repaint.
        if !self.forge_status.is_ready() {
            return;
        }
        for url in self.workspace_pull_request_urls() {
            let Some(writer) = self.pull_request_states
                                   .claim_refresh(url.clone(), pull_request_state::MAX_AGE)
            else {
                continue;
            };
            self.runtime.spawn_blocking(move || {
                            let runner = GhRunner::new();
                            writer.record(knot_forge::pull_request_state_with(&runner, &url).ok());
                        });
        }
    }

    /// Forget the state of every URL this workspace no longer records, so the
    /// cache does not grow with every pull request the window has ever shown.
    pub(super) fn prune_pull_request_states(&mut self) {
        let live = self.workspace_pull_request_urls();
        self.pull_request_states.retain(|url| live.contains(url));
    }

    /// Open a row's pull request in the user's browser.
    pub(super) fn open_pull_request(&mut self, url: &str) {
        if !crate::open_in::open_url(url) {
            // Reported rather than swallowed: a row that did nothing on a
            // click is indistinguishable from one that is broken. The record
            // stays and the view stays open, per the spec.
            self.pull_request_open_failed = true;
        }
    }

    /// Ask before forgetting a row, then forget it.
    ///
    /// Confirmed because it is destructive to Knot's own record and cannot be
    /// undone except by the agent printing the URL again - see
    /// `.claude/rules/knot-ui-conventions.md`. Nothing on the forge changes,
    /// which is what the prompt says.
    pub(super) fn confirm_remove_pull_request(&mut self, agent_id: Uuid, url: String,
                                              window: &mut Window, cx: &mut Context<Self>) {
        let body = knot_core::l10n::t_with("pull_requests.remove_body", &[("url", &url)]);
        let entity = cx.entity();
        window.open_alert_dialog(cx, move |alert, _, _| {
                  let entity = entity.clone();
                  let url = url.clone();
                  alert.title(knot_core::l10n::t("pull_requests.remove_title"))
                       .description(body.clone())
                       .confirm()
                       .on_ok(move |_, _, app| {
                           entity.update(app, |view, cx| {
                                     view.remove_pull_request(agent_id, &url);
                                     cx.notify();
                                 });
                           true
                       })
              });
    }

    /// Forget one recorded pull request. Knot's record only.
    pub(super) fn remove_pull_request(&mut self, agent_id: Uuid, url: &str) {
        let removed = self.store.lock().remove_pull_request(agent_id, url);
        if removed {
            self.persist_pull_requests();
            self.prune_pull_request_states();
        }
    }
}
