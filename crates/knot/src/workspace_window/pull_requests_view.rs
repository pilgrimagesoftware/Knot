//! What the Pull Requests view asks of the window: the rows to draw, the
//! breakdown the sidebar row shows, the refreshes that fill them in, and the
//! two things the user can do with a row.
//!
//! Contract: the `pull-request-tracking` capability spec under
//! `openspec/changes/pull-request-tracking/specs/`.
//!
//! Recording is `super::pull_requests`; this is the reading half.

use std::time::SystemTime;

use gpui_kit::App;
use gpui_kit::component::WindowExt;
use gpui_kit::{Context, Window};
use knot_forge::GhRunner;
use uuid::Uuid;

use super::WorkspaceWindow;
use crate::pull_request_groups;
use crate::pull_request_state::{self, PullRequestCounts, PullRequestLookup};
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::render::pull_requests_pane::{PullRequestGroup, PullRequestRow};

impl WorkspaceWindow {
    /// This workspace's recorded pull request URLs, newest first, each once.
    ///
    /// Once per pull request rather than per record: the same pull request
    /// recorded by six agents is one pull request to count and one to fetch.
    pub(super) fn workspace_pull_request_urls(&self) -> Vec<String> {
        pull_request_groups::unique_urls(&self.store
                                              .lock()
                                              .pull_requests_for_workspace(self.workspace_id))
    }

    /// How this workspace's records break down by state, for the sidebar row.
    pub(super) fn pull_request_counts(&self) -> PullRequestCounts {
        pull_request_state::counts_for(&self.pull_request_states.snapshot(),
                                       &self.workspace_pull_request_urls())
    }

    /// The rows the pane draws, one per pull request, grouped by the agents
    /// that opened them.
    ///
    /// Flattened out of the store and the cache here, so neither lock is held
    /// while the element tree is built.
    pub(super) fn pull_request_groups(&self) -> Vec<PullRequestGroup> {
        let states = self.pull_request_states.snapshot();
        let store = self.store.lock();
        // An agent that has gone owns nothing: its records go with it, so a
        // record naming one is only reachable mid-removal.
        let records = store.pull_requests_for_workspace(self.workspace_id)
                           .into_iter()
                           .filter(|record| store.agent(record.agent_id).is_some())
                           .collect::<Vec<_>>();
        let mut groups = Vec::new();
        for group in pull_request_groups::group_records(&records) {
            let mut names = group.owners
                                 .iter()
                                 .filter_map(|id| store.agent(*id))
                                 .map(|agent| agent.name.clone())
                                 .collect::<Vec<_>>();
            names.sort();
            let rows = group.urls
                            .into_iter()
                            .map(|url| {
                                let lookup = states.get(&url).cloned();
                                PullRequestRow { url, lookup }
                            })
                            .collect();
            groups.push(PullRequestGroup { agent_ids: group.owners,
                                           agents: names.join(", "),
                                           rows });
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
    pub(super) fn refresh_pull_request_states(&mut self, cx: &App) {
        if self.view_mode != WorkspaceViewMode::PullRequests {
            return;
        }
        // Before the fetches rather than after: this frame's answers landed
        // on an earlier one, and a record about to be dropped should not be
        // re-fetched on the way out.
        self.expire_merged_pull_requests(cx);
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
            // A pull request the forge says does not exist is asked about
            // once per window, not once per cycle; see
            // `PullRequestLookup::is_final`.
            if self.pull_request_states
                   .holds(&url, PullRequestLookup::is_final)
            {
                continue;
            }
            let Some(writer) = self.pull_request_states
                                   .claim_refresh(url.clone(), pull_request_state::MAX_AGE)
            else {
                continue;
            };
            self.runtime.spawn_blocking(move || {
                            let runner = GhRunner::new();
                            writer.record(knot_forge::pull_request_state_with(&runner, &url).into());
                        });
        }
    }

    /// Whether any record has passed the retention window and is waiting for
    /// a frame to be dropped in.
    ///
    /// Read from [`Self::repaint_poll_tick`]'s chain, because expiry happens
    /// on the render path and an idle window does not render. The states this
    /// reads land from `spawn_blocking`, and a re-fetch that returns the same
    /// answer does not flag the cache as changed - so a merged pull request
    /// sitting stable across the 24-hour boundary produces no repaint of its
    /// own, and without this the row would wait for something unrelated to
    /// happen. On a workspace with nothing running, that could be never.
    ///
    /// Pure: it clears nothing, so the `||` chain may short-circuit past it
    /// without stranding anything. The removal itself stays in
    /// [`Self::expire_merged_pull_requests`], which this only schedules a
    /// frame for.
    ///
    /// Gated on the view before it touches the store, so the common tick -
    /// the view closed - is one enum comparison.
    pub(super) fn pull_requests_expiring(&self) -> bool {
        if self.view_mode != WorkspaceViewMode::PullRequests {
            return false;
        }
        !pull_request_state::expired_urls(&self.pull_request_states.snapshot(),
                                          &self.workspace_pull_request_urls(),
                                          pull_request_state::MERGED_RETENTION,
                                          SystemTime::now()).is_empty()
    }

    /// Drop the records whose pull requests merged longer ago than the
    /// retention window.
    ///
    /// Runs from the refresh cycle, so it is gated on the Pull Requests view
    /// being open exactly as the fetch it depends on is: expiry is decided
    /// from fetched state, and fetching off-view is what that gate exists to
    /// prevent. A record that passes the window while the view is closed is
    /// therefore dropped when the user next opens it - the only moment a
    /// stale row costs them anything.
    ///
    /// No confirmation, unlike [`Self::confirm_remove_pull_request`]: that
    /// prompt exists because the user asked for something destructive, and
    /// prompting for something they did not do is noise.
    ///
    /// The early return is what keeps the common frame free. Expiring is at
    /// most a once-a-day event per record, and only then does this write the
    /// document - the same shape as recording a new sighting, which also
    /// persists from a frame when something actually changed.
    fn expire_merged_pull_requests(&mut self, cx: &App) {
        let expired = pull_request_state::expired_urls(&self.pull_request_states.snapshot(),
                                                       &self.workspace_pull_request_urls(),
                                                       pull_request_state::MERGED_RETENTION,
                                                       SystemTime::now());
        if expired.is_empty() {
            return;
        }
        // Bound rather than locked in the `if` condition, matching
        // `remove_pull_request`: `persist_pull_requests` takes the same
        // non-reentrant lock, so the guard has to be gone before it runs. A
        // plain `if` would drop it at the end of the condition and an `if let`
        // would not, which is too fine a distinction to rest a frozen window
        // on - and nothing here can be unit-tested, since it needs GPUI.
        let forgotten = self.store.lock().forget_pull_requests(&expired);
        if forgotten {
            self.persist_pull_requests(cx);
            self.prune_pull_request_states();
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
    ///
    /// `agent_ids` is every agent the row is listed for, so one row on
    /// screen is one removal.
    pub(super) fn confirm_remove_pull_request(&mut self, agent_ids: Vec<Uuid>, url: String,
                                              window: &mut Window, cx: &mut Context<Self>) {
        let body = knot_core::l10n::t_with("pull_requests.remove_body", &[("url", &url)]);
        let entity = cx.entity();
        window.open_alert_dialog(cx, move |alert, _, _| {
                  let entity = entity.clone();
                  let url = url.clone();
                  let agent_ids = agent_ids.clone();
                  alert.title(knot_core::l10n::t("pull_requests.remove_title"))
                       .description(body.clone())
                       .confirm()
                       .on_ok(move |_, _, app| {
                           entity.update(app, |view, cx| {
                                     view.remove_pull_request(&agent_ids, &url, cx);
                                     cx.notify();
                                 });
                           true
                       })
              });
    }

    /// Forget one listed pull request for every agent it is attributed to.
    /// Knot's record only.
    ///
    /// Every attribution rather than one: dropping a single agent's record
    /// would make the row the user just removed reappear under the remaining
    /// agent's heading.
    pub(super) fn remove_pull_request(&mut self, agent_ids: &[Uuid], url: &str, cx: &App) {
        // A loop rather than `any`, which would stop at the first agent it
        // removed a record for and leave the rest listed.
        let mut removed = false;
        {
            let mut store = self.store.lock();
            for agent_id in agent_ids {
                removed |= store.remove_pull_request(*agent_id, url);
            }
        }
        if removed {
            self.persist_pull_requests(cx);
            self.prune_pull_request_states();
        }
    }
}
